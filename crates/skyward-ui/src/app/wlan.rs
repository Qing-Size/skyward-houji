//! The WLAN pages: the networks nearby, joining one with its password,
//! adding one by hand, and what the status bar shows of the one joined.

use super::*;

/// A password typed, which what is printed of it leaves out: the recovery
/// prints each effect the pages ask for.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret({} characters)", self.0.chars().count())
    }
}

/// A network being added by hand, as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adding {
    pub name: String,
    pub security: Security,
    /// What the keyboard types into.
    pub field: Field,
    /// The protections to choose from are open over the page.
    pub choosing: bool,
}

/// A text field of the page that adds a network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Password,
    /// 手动 IP 页的四格。2026-11-27。
    Address,
    Mask,
    Gateway,
    Dns,
}

impl crate::settings::StaticV4 {
    /// 解析页上填的四格。掩码留空按 /24；DNS 可空（空则用网关），逗号或
    /// 空格分隔多个。网关必须填：填错时告诉是哪一格填坏了。
    /// 2026-11-27。
    pub fn parsed(&self) -> Result<(std::net::Ipv4Addr, std::net::Ipv4Addr, std::net::Ipv4Addr, Vec<std::net::Ipv4Addr>), Bad> {
        let address: std::net::Ipv4Addr = self.address.trim().parse().map_err(|_| Bad::Address)?;
        let mask: std::net::Ipv4Addr = match self.mask.trim() {
            "" => std::net::Ipv4Addr::new(255, 255, 255, 0),
            text => text.parse().map_err(|_| Bad::Mask)?,
        };
        let gateway: std::net::Ipv4Addr = self.gateway.trim().parse().map_err(|_| Bad::Gateway)?;
        // 网关得和地址在同一网段：182.168.5.1 配 192.168.5.60/24 时内核会
        // 直接拒绝加路由（Network is unreachable），以前就这么存进去了。
        // 2026-11-27。
        let net = u32::from(address) & u32::from(mask);
        if address == gateway || u32::from(gateway) & u32::from(mask) != net {
            return Err(Bad::GatewayOutOfSubnet);
        }
        let mut dns = Vec::new();
        for part in self.dns.split([' ', ',', ';']).filter(|p| !p.is_empty()) {
            dns.push(part.parse().map_err(|_| Bad::Dns)?);
        }
        if dns.is_empty() {
            dns.push(gateway);
        }
        Ok((address, mask, gateway, dns))
    }

    /// 打开页时先把现在这套填上：已经连着的时候，只改地址那一格就够了。
    pub fn seed(&mut self, link: &skyward_net::wlan::Link) {
        if self.address.trim().is_empty() {
            self.address = link.address.to_string();
        }
        if self.mask.trim().is_empty() {
            self.mask = link.mask.to_string();
        }
        if self.gateway.trim().is_empty()
            && let Some(router) = link.router
        {
            self.gateway = router.to_string();
        }
        if self.dns.trim().is_empty() && !link.dns.is_empty() {
            self.dns = link.dns.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ");
        }
    }
}

/// 手动地址哪一格填坏了。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bad {
    Address,
    Mask,
    Gateway,
    /// 网关和地址不在同一网段，或者和地址一样。
    GatewayOutOfSubnet,
    Dns,
}

/// How a network added by hand can be protected, in the order the stock
/// list gives them: those the recovery joins.
pub const ADDABLE: [Security; 3] = [Security::Open, Security::Psk, Security::Sae];

/// The longest name a network can have, in bytes.
pub(super) const SSID_MAX: usize = 32;

/// What turns down a password the protection `security` cannot take.
pub(super) fn refused(l: Lang, security: Security, password: &str) -> Option<&'static str> {
    match security {
        Security::Psk | Security::PskSae if !(8..=63).contains(&password.len()) => Some(l.pick(texts::toast::PASSWORD_LENGTH)),
        _ if password.is_empty() => Some(l.pick(texts::toast::ENTER_PASSWORD)),
        _ => None,
    }
}

impl App {
    /// How the WLAN stands, as the runtime last told it.
    pub fn set_wlan(&mut self, w: Wlan) {
        if self.state.wlan.as_ref() == Some(&w) {
            return;
        }
        self.state.wlan = Some(w);
        if matches!(self.screen(), Screen::Home | Screen::Wlan | Screen::WlanNetwork | Screen::Update | Screen::SkywardUpdate) {
            self.refresh();
        }
        // The status bar shows the network joined over every page.
        self.dirty = true;
    }

    /// How fast data went over the network joined in the last second, which
    /// only the status bar shows.
    pub fn set_traffic(&mut self, t: Traffic) {
        if self.state.traffic != t {
            self.state.traffic = t;
            self.dirty = true;
        }
    }

    /// What the status bar shows of the WLAN: the network joined, while it
    /// has an address.
    pub fn wlan_mark(&self) -> Option<WlanMark> {
        let Some(Joined { stage: Joining::Connected(link), .. }) = self.state.wlan.as_ref()?.joined.as_ref() else { return None };
        let t = self.state.traffic;
        let (speed, unit) = fmt::speed(t.rx + t.tx);
        Some(WlanMark { level: fmt::signal_level(link.signal), receiving: t.rx > 0, sending: t.tx > 0, speed, unit })
    }

    /// Open the password page for `network`, empty.
    pub(super) fn ask_password(&mut self, network: Network) {
        self.state.joining = Some(network);
        self.state.password.clear();
        self.state.show_password = false;
        self.state.layer = Layer::Letters;
        self.open(Screen::WlanPassword);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::color::over;

    #[test]
    fn a_manual_address_is_read_off_the_page_and_empty_dns_takes_the_gateway() {
        let set = crate::settings::StaticV4 {
            address: "192.168.1.23".into(),
            mask: String::new(),
            gateway: "192.168.1.1".into(),
            dns: String::new(),
        };
        let (address, mask, gateway, dns) = set.parsed().expect("the address reads");
        assert_eq!(address.to_string(), "192.168.1.23");
        assert_eq!(mask.to_string(), "255.255.255.0", "an empty mask is a /24");
        assert_eq!(dns, vec![gateway], "an empty DNS takes the gateway");
        let many = crate::settings::StaticV4 { dns: "1.1.1.1, 8.8.8.8".into(), ..set };
        assert_eq!(many.parsed().expect("read").3.len(), 2);
        let bad = crate::settings::StaticV4 { address: "192.168.1".into(), gateway: "192.168.1.1".into(), ..Default::default() };
        assert_eq!(bad.parsed(), Err(Bad::Address), "a short address names the field");
        let no_gateway = crate::settings::StaticV4 { address: "192.168.1.23".into(), ..Default::default() };
        assert_eq!(no_gateway.parsed(), Err(Bad::Gateway), "a missing gateway names the field");
        let bad_dns = crate::settings::StaticV4 { address: "192.168.1.23".into(), gateway: "192.168.1.1".into(), dns: "1.1.1".into(), ..Default::default() };
        assert_eq!(bad_dns.parsed(), Err(Bad::Dns));
        // 182.168.5.1 配 192.168.5.60/24：内核会拒绝加路由，必须在这里拦下。
        let off_subnet = crate::settings::StaticV4 {
            address: "192.168.5.60".into(),
            mask: "255.255.255.0".into(),
            gateway: "182.168.5.1".into(),
            dns: String::new(),
        };
        assert_eq!(off_subnet.parsed(), Err(Bad::GatewayOutOfSubnet));
    }

    #[test]
    fn opening_the_address_page_seeds_it_from_the_lease_in_use() {
        use std::net::Ipv4Addr;
        let mut set = crate::settings::StaticV4::default();
        let link = skyward_net::wlan::Link {
            address: Ipv4Addr::new(192, 168, 5, 60),
            mask: Ipv4Addr::new(255, 255, 255, 0),
            router: Some(Ipv4Addr::new(192, 168, 5, 1)),
            dns: vec![Ipv4Addr::new(192, 168, 5, 1)],
            signal: -50,
            frequency: 5180,
            speed: None,
            generation: None,
            mac: "aa:bb:cc:dd:ee:ff".into(),
            reach: None,
            by_hand: false,
        };
        set.seed(&link);
        assert_eq!(set.gateway, "192.168.5.1", "the gateway is filled in, not left blank");
        assert_eq!(set.mask, "255.255.255.0");
        assert_eq!(set.dns, "192.168.5.1");
        // 已经填过的不覆盖。
        set.seed(&link);
        assert_eq!(set.gateway, "192.168.5.1");
    }

    #[test]
    fn the_address_page_takes_a_key_and_the_return_key_applies_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan::default());
        app.act(Action::OpenWlanStatic, 0);
        assert_eq!(app.screen(), Screen::WlanStatic);
        // 键盘上的任何键以前都会撞到「这一页没有键盘」那道 unreachable，
        // 整个 recovery 跟着 panic（2026-11-27，真机闪退）。
        app.act(Action::Type(Cap::Char('1')), 0);
        assert_eq!(app.state.static_ip.address, "1");
        app.state.static_ip = crate::settings::StaticV4 {
            address: "192.168.1.23".into(),
            mask: "255.255.255.0".into(),
            gateway: "192.168.1.1".into(),
            dns: String::new(),
        };
        let fx = app.act(Action::Type(Cap::Enter), 0);
        assert!(
            fx.iter().any(|e| matches!(e, Effect::WlanStatic { .. })),
            "回车就是「应用」"
        );
        // 页画得出来：四格排成两行，键盘之上不越界。
        let _ = app.frame(&f, &i);
    }

    #[test]
    fn deleting_the_network_joined_also_forgets_it() {
        use std::net::Ipv4Addr;
        let mut app = App::new(Screen::Home, W, H, PPD);
        let ssid = b"Sarva5".to_vec();
        let link = skyward_net::wlan::Link {
            address: Ipv4Addr::new(192, 168, 5, 60),
            mask: Ipv4Addr::new(255, 255, 255, 0),
            router: Some(Ipv4Addr::new(192, 168, 5, 1)),
            dns: vec![],
            signal: -50,
            frequency: 5180,
            speed: None,
            generation: None,
            mac: "aa:bb:cc:dd:ee:ff".into(),
            reach: None,
            by_hand: false,
        };
        app.state.wlan = Some(Wlan {
            joined: Some(Joined { ssid: ssid.clone(), name: "Sarva5".into(), security: Security::Psk, stage: Joining::Connected(link) }),
            ..Wlan::default()
        });
        app.state.wlan_saved = vec![SavedWlan { ssid: ssid.clone(), name: "Sarva5".into(), security: Security::Psk }];
        app.state.details = Some(Network { ssid, name: "Sarva5".into(), signal: -50, frequency: 5180, bands: Default::default(), security: Security::Psk });
        app.open(Screen::WlanNetwork);
        let fx = app.act(Action::LeaveNetwork, 0);
        assert!(
            fx.iter().any(|e| matches!(e, Effect::ForgetKept { .. })),
            "the saved network is taken off the list"
        );
        assert!(
            fx.iter().any(|e| matches!(e, Effect::LeaveWlan)),
            "and the one joined is left as well"
        );
    }

    #[test]
    fn a_blank_address_page_names_the_field_and_leaves_dhcp_alone() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan::default());
        app.act(Action::OpenWlanStatic, 0);
        app.state.static_field = Field::Dns;
        let fx = app.act(Action::ApplyStatic, 0);
        assert!(!fx.iter().any(|e| matches!(e, Effect::WlanStatic { .. })), "an unread page applies nothing");
        assert!(!app.state.static_on, "DHCP stays the mode");
        assert_eq!(app.state.static_field, Field::Address, "the keyboard goes to the field that is wrong");
        // 已经是自动获取，再点「自动获取」什么都不发。
        assert!(app.act(Action::UseDhcp, 0).is_empty());
    }

    #[test]
    fn the_wlan_page_joins_a_network_with_its_password_and_leaves_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let entry = |app: &mut App| app.frame(&f, &i).hits.iter().any(|h| h.action == Action::Open(Screen::Wlan));
        assert!(!entry(&mut app), "a phone whose WLAN is not brought up has no WLAN");
        app.state.wlan = Some(Wlan::default());
        app.refresh();
        assert!(entry(&mut app));
        assert!(app.act(Action::Open(Screen::Wlan), 0).is_empty(), "a WLAN that is off is not scanned");
        assert_eq!(tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)), vec![Effect::Wlan(true)]);
        assert_eq!(app.state.wlan.as_ref().unwrap().power, Power::Starting, "the switch moves at once");
        let nearby = vec![network("Skyward", -34, Security::Psk), network("Skyward-Open", -72, Security::Open), network("Skyward-WEP", -60, Security::Wep)];
        app.set_wlan(Wlan { power: Power::On, networks: nearby, joined: None });
        tap(&mut app, &f, &i, Action::JoinNetwork(2));
        assert_eq!(toast(&app), Some("暂不支持 WEP 加密的网络"));
        let cafe = Effect::JoinWlan { ssid: b"Skyward-Open".to_vec(), name: "Skyward-Open".into(), security: Security::Open, password: Secret::default() };
        assert_eq!(tap(&mut app, &f, &i, Action::JoinNetwork(1)), vec![cafe], "an open network is joined at once");
        tap(&mut app, &f, &i, Action::JoinNetwork(0));
        assert_eq!(app.screen(), Screen::WlanPassword);
        let type_in = |app: &mut App, text: &str| {
            for ch in text.chars() {
                tap(app, &f, &i, Action::Type(Cap::Char(ch)));
            }
        };
        type_in(&mut app, "secret");
        assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
        assert_eq!(toast(&app), Some("请输入 8 到 63 位密码"));
        tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Digits)));
        type_in(&mut app, "12");
        let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
        let join = Effect::JoinWlan { ssid: b"Skyward".to_vec(), name: "Skyward".into(), security: Security::Psk, password: Secret("secret12".into()) };
        assert_eq!(format!("{join:?}"), r#"JoinWlan { ssid: [83, 107, 121, 119, 97, 114, 100], name: "Skyward", security: Psk, password: Secret(8 characters) }"#, "what is printed leaves the password out");
        assert_eq!(fx, vec![join]);
        assert_eq!(app.screen(), Screen::Wlan, "the WLAN page shows how joining goes");
        assert_eq!((app.state.password.as_str(), app.state.joining.as_ref()), ("", None), "the password is not kept");
        app.set_wlan(connected("Skyward"));
        assert_eq!(fmt::wlan_value(Lang::Hans, app.state.wlan.as_ref().unwrap()), "Skyward");
        tap(&mut app, &f, &i, Action::OpenJoined);
        assert_eq!(app.screen(), Screen::WlanNetwork);
        assert_eq!(tap(&mut app, &f, &i, Action::LeaveNetwork), vec![Effect::LeaveWlan]);
        assert_eq!(app.screen(), Screen::Wlan);
        assert_eq!(tap(&mut app, &f, &i, Action::Toggle(Switch::Wlan)), vec![Effect::Wlan(false)]);
        assert_eq!(app.state.wlan, Some(Wlan::default()));
    }

    #[test]
    fn a_refused_password_is_asked_for_again_from_the_networks_row() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(joined("Skyward", Joining::WrongPassword));
        app.act(Action::Open(Screen::Wlan), 0);
        tap(&mut app, &f, &i, Action::OpenJoined);
        assert_eq!(app.screen(), Screen::WlanPassword);
        assert_eq!(app.state.joining.as_ref().map(|n| n.name.as_str()), Some("Skyward"));
        app.key(Key::Back, 0);
        app.set_wlan(joined("Skyward", Joining::Joining));
        tap(&mut app, &f, &i, Action::OpenJoined);
        assert_eq!(app.screen(), Screen::WlanNetwork, "a network being joined shows how that goes");
        assert_eq!(app.state.details.as_ref().map(|n| n.name.as_str()), Some("Skyward"));
        app.key(Key::Back, 0);
        assert_eq!(app.state.details, None);
    }

    #[test]
    fn the_status_bar_shows_the_network_joined_and_how_fast_data_goes_without_the_page_drawn_again() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.set_wlan(Wlan { power: Power::On, networks: Vec::new(), joined: None });
        assert!(app.wlan_mark().is_none(), "no network joined");
        app.set_wlan(joined("Skyward", Joining::Addressing));
        assert!(app.wlan_mark().is_none(), "not while it has no address");
        app.set_wlan(connected("Skyward"));
        let mark = app.wlan_mark().unwrap();
        assert_eq!((mark.level, mark.speed.as_str(), mark.unit, mark.receiving, mark.sending), (3, "0.00", "KB/s", false, false));
        app.frame(&f, &i);
        app.dirty = false;
        app.set_traffic(Traffic { rx: 42_000, tx: 0 });
        assert!(app.dirty && app.frame.is_some(), "the status bar is drawn again, and the page as it was");
        let mark = app.wlan_mark().unwrap();
        assert_eq!((mark.speed.as_str(), mark.receiving, mark.sending), ("41.0", true, false));
    }

    #[test]
    fn a_network_is_added_by_hand_with_its_name_protection_and_password() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan { power: Power::On, networks: Vec::new(), joined: None });
        app.act(Action::Open(Screen::Wlan), 0);
        tap(&mut app, &f, &i, Action::AddNetwork);
        assert_eq!(app.screen(), Screen::WlanAdd, "the list ends with adding one, found networks or none");
        let type_in = |app: &mut App, text: &str| {
            for ch in text.chars() {
                tap(app, &f, &i, Action::Type(Cap::Char(ch)));
            }
        };
        assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
        assert_eq!(toast(&app), Some("请输入网络名称"));
        type_in(&mut app, "skyward");
        let frame = app.frame(&f, &i);
        assert!(!frame.hits.iter().any(|h| h.action == Action::TypeInto(Field::Password)), "an open network takes no password");
        // The protections open over the page, which then answers no touch
        // but closes them.
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        let frame = app.frame(&f, &i);
        assert!(frame.hits.iter().all(|h| matches!(h.action, Action::CloseChoices | Action::PickSecurity(_))));
        assert!(frame.caret.is_none(), "no cursor shows through the list");
        let below = (W as f32 / 2.0, H as f32 - 20.0);
        app.touch(Touch::Down(below.0, below.1), &f, 0);
        app.touch(Touch::Up, &f, 0);
        let a = app.state.adding.clone().unwrap();
        assert_eq!((a.name.as_str(), a.security, a.choosing), ("skyward", Security::Open, false), "a touch off the list types nothing");
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        app.key(Key::Back, 0);
        assert_eq!((app.screen(), app.state.adding.as_ref().unwrap().choosing), (Screen::WlanAdd, false), "back closes the list first");
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        tap(&mut app, &f, &i, Action::PickSecurity(Security::Psk));
        let a = app.state.adding.clone().unwrap();
        assert_eq!((a.security, a.choosing, a.field), (Security::Psk, false, Field::Name));
        // The keyboard types into the field touched.
        tap(&mut app, &f, &i, Action::TypeInto(Field::Password));
        type_in(&mut app, "secret");
        tap(&mut app, &f, &i, Action::TypeInto(Field::Name));
        assert_eq!((app.state.adding.as_ref().unwrap().name.as_str(), app.state.password.as_str()), ("skyward", "secret"));
        assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
        assert_eq!(toast(&app), Some("请输入 8 到 63 位密码"));
        assert_eq!(app.state.adding.as_ref().unwrap().field, Field::Password, "the keyboard types into the field turned down");
        tap(&mut app, &f, &i, Action::Type(Cap::Show(Layer::Digits)));
        type_in(&mut app, "12");
        let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
        let join = Effect::JoinWlan { ssid: b"skyward".to_vec(), name: "skyward".into(), security: Security::Psk, password: Secret("secret12".into()) };
        assert_eq!(fx, vec![join]);
        assert_eq!(app.screen(), Screen::Wlan, "the WLAN page shows how joining goes");
        assert_eq!((app.state.adding.as_ref(), app.state.password.as_str()), (None, ""), "nothing typed is kept");
    }

    #[test]
    fn a_network_added_by_hand_is_turned_down_for_a_name_too_long_and_joined_open_without_its_password() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan { power: Power::On, networks: vec![network("Skyward-Open", -72, Security::Open)], joined: None });
        app.act(Action::Open(Screen::Wlan), 0);
        tap(&mut app, &f, &i, Action::AddNetwork);
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        tap(&mut app, &f, &i, Action::PickSecurity(Security::Sae));
        tap(&mut app, &f, &i, Action::TypeInto(Field::Password));
        tap(&mut app, &f, &i, Action::Type(Cap::Char('x')));
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        tap(&mut app, &f, &i, Action::PickSecurity(Security::Open));
        assert_eq!(app.state.adding.as_ref().unwrap().field, Field::Name, "with the password field gone, the keyboard types the name");
        for _ in 0..33 {
            tap(&mut app, &f, &i, Action::Type(Cap::Char('a')));
        }
        assert!(tap(&mut app, &f, &i, Action::Type(Cap::Enter)).is_empty());
        assert_eq!(toast(&app), Some("网络名称不能超过 32 个字符"));
        tap(&mut app, &f, &i, Action::Type(Cap::Backspace));
        let fx = tap(&mut app, &f, &i, Action::Type(Cap::Enter));
        let name = "a".repeat(32);
        assert_eq!(fx, vec![Effect::JoinWlan { ssid: name.clone().into_bytes(), name, security: Security::Open, password: Secret::default() }]);
    }

    #[test]
    fn the_keys_open_a_list_of_choices_take_one_and_close_it_going_back() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan { power: Power::On, networks: Vec::new(), joined: None });
        app.act(Action::Open(Screen::Wlan), 0);
        app.act(Action::AddNetwork, 0);
        app.frame(&f, &i);
        light_on(&mut app, &f, &i, Stop::Hit(Action::ChooseSecurity));
        press(&mut app, Key::Power, 0);
        assert!(app.state.adding.as_ref().unwrap().choosing);
        let t = app.theme();
        let stops: Vec<Stop> = focus::spots(&t, app.frame(&f, &i)).iter().map(|s| s.stop).collect();
        let pick = |s| Stop::Hit(Action::PickSecurity(s));
        assert_eq!(stops, [Stop::Back, pick(Security::Open), pick(Security::Psk), pick(Security::Sae)], "only the list and back");
        light_on(&mut app, &f, &i, pick(Security::Sae));
        press(&mut app, Key::Power, 0);
        let a = app.state.adding.clone().unwrap();
        assert_eq!((a.security, a.choosing), (Security::Sae, false));
        light_on(&mut app, &f, &i, Stop::Hit(Action::ChooseSecurity));
        press(&mut app, Key::Power, 0);
        press(&mut app, Key::Back, 0);
        assert_eq!((app.screen(), app.state.adding.as_ref().unwrap().choosing), (Screen::WlanAdd, false));
    }

    #[test]
    fn a_choice_shows_held_down_while_touched() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.wlan = Some(Wlan { power: Power::On, networks: Vec::new(), joined: None });
        app.act(Action::Open(Screen::Wlan), 0);
        tap(&mut app, &f, &i, Action::AddNetwork);
        tap(&mut app, &f, &i, Action::ChooseSecurity);
        let wpa3 = Action::PickSecurity(Security::Sae);
        let rect = app.frame(&f, &i).hits.iter().find(|h| h.action == wpa3).unwrap().rect;
        // Clear of the words, where only the list's own colour shows.
        let at = |app: &mut App| {
            let c = &app.frame(&f, &i).canvas;
            let (x, y) = ((rect.x + rect.w) as u32 - 20, (rect.y + rect.h / 2.0) as u32);
            let k = ((y - c.held().start) * c.w + x) as usize * 4;
            Rgba::rgb(c.data[k], c.data[k + 1], c.data[k + 2])
        };
        let t = app.theme();
        assert_eq!(at(&mut app), t.pal.popup);
        let (x, y) = spot(&mut app, &f, &i, wpa3);
        app.touch(Touch::Down(x, y), &f, 0);
        assert_eq!(app.state.held, Some(wpa3));
        assert_eq!(at(&mut app), over(t.pal.row_press, t.pal.popup));
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.state.held, None);
        assert_eq!(app.state.adding.as_ref().unwrap().security, Security::Sae);
    }

    #[test]
    fn a_networks_round_button_opens_its_details_and_its_row_joins_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let nearby = vec![network("Skyward", -34, Security::Psk), network("Skyward-Open", -72, Security::Open)];
        app.state.wlan = Some(Wlan { power: Power::On, networks: nearby, joined: None });
        app.act(Action::Open(Screen::Wlan), 0);
        assert_eq!(tap(&mut app, &f, &i, Action::Rescan), vec![Effect::ScanWlan]);
        tap(&mut app, &f, &i, Action::NetworkDetails(1));
        assert_eq!(app.screen(), Screen::WlanNetwork);
        assert_eq!(app.state.details.as_ref().map(|n| n.name.as_str()), Some("Skyward-Open"));
        assert!(!app.frame(&f, &i).hits.iter().any(|h| h.action == Action::LeaveNetwork), "a network not chosen has nothing to forget");
        app.key(Key::Back, 0);
        // The round button sits on its row, and is what a touch on it finds.
        let fr = app.frame(&f, &i);
        let row = fr.hits.iter().find(|h| h.action == Action::JoinNetwork(1)).expect("the row").rect;
        let round = fr.hits.iter().find(|h| h.action == Action::NetworkDetails(1)).expect("its round button").rect;
        assert!(row.contains(round.x + round.w / 2.0, round.y + round.h / 2.0));
        assert_eq!(fr.hit(round.x + round.w / 2.0, round.y + round.h / 2.0).map(|h| h.action), Some(Action::NetworkDetails(1)));
        assert_eq!(fr.hit(row.x + row.w / 3.0, row.y + row.h / 2.0).map(|h| h.action), Some(Action::JoinNetwork(1)));
    }

    #[test]
    fn the_address_page_takes_the_pinyin_bar_too() {
        use crate::app::testing::*;
        use crate::app::{Action, App, Screen};
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.act(Action::OpenWlanStatic, 0);
        assert!(app.text_context(), "手动 IP 页也能切中/EN");
        assert!(app.ime_context(), "这一版带拼音输入");
    }
}
