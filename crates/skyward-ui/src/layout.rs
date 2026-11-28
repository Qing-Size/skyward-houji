//! 布局：首页的分组、顺序、名字，以及控制中心的磁贴，写出一个文件，也能从
//! 文件读回来。2026-11-27。
//!
//! 文件是 JSON，`kind` 一项说明它是布局，别人的 JSON 不往里导。

use crate::app::{from_ids, normalize, Entry, Section, State};
use crate::settings::HomeSection;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 文件里认自己是不是布局。
pub const KIND: &str = "skyward-layout";
/// 这一版写到文件里的版本号。
pub const VERSION: u32 = 1;

/// 一份布局：首页怎么分栏、条目怎么排、各自叫什么，以及磁贴摆了哪几枚。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub kind: String,
    pub version: u32,
    /// 首页顶上那个大字。
    pub home_title: String,
    /// 自建的分类。
    pub home_sections: Vec<HomeSection>,
    /// 单独放在首页最上面的条目，按 `Entry::id()` 写。
    pub home_top: Vec<String>,
    /// 「其他」那一栏的顺序。
    pub home_rest: Vec<String>,
    /// 「其他」的名字，以及它在各栏之间的位置。
    pub home_rest_name: String,
    pub home_rest_at: Option<usize>,
    pub home_rest_collapsed: bool,
    /// 条目自己起的名字：`Entry::id()` → 名字。
    pub entry_names: BTreeMap<String, String>,
    /// 控制中心摆了哪几枚磁贴，按 `Tile::id()` 写。
    pub tiles: Vec<String>,
}

impl Layout {
    /// 现在这一份布局。
    pub fn of(state: &State) -> Layout {
        Layout {
            kind: KIND.to_string(),
            version: VERSION,
            home_title: state.home_title.clone(),
            home_sections: state
                .home_sections
                .iter()
                .map(|section| HomeSection {
                    name: section.name.clone(),
                    entries: section.entries.iter().map(|entry| entry.id().to_string()).collect(),
                    icon: section.icon.id().to_string(),
                    collapsed: section.collapsed,
                })
                .collect(),
            home_top: state.home_top.iter().map(|entry| entry.id().to_string()).collect(),
            home_rest: state.home_rest.iter().map(|entry| entry.id().to_string()).collect(),
            home_rest_name: state.home_rest_name.clone(),
            home_rest_at: state.home_rest_at,
            home_rest_collapsed: state.home_rest_collapsed,
            entry_names: state.entry_names.clone(),
            tiles: state.tiles.iter().map(|tile| tile.id().to_string()).collect(),
        }
    }

    /// 把这一份布局按上去：认不出的条目与磁贴丢掉，一条不剩时按原来的来。
    pub fn apply(&self, state: &mut State) {
        state.home_title = self.home_title.clone();
        state.entry_names = self.entry_names.clone();
        state.home_rest_name = self.home_rest_name.clone();
        state.home_rest_collapsed = self.home_rest_collapsed;
        let mut sections: Vec<Section> = self
            .home_sections
            .iter()
            .map(|section| Section {
                name: section.name.clone(),
                entries: section.entries.iter().filter_map(|id| Entry::from_id(id)).collect(),
                icon: crate::icons::Icon::from_id(&section.icon).unwrap_or(crate::icons::Icon::Layers),
                collapsed: section.collapsed,
            })
            .collect();
        let mut top: Vec<Entry> = self.home_top.iter().filter_map(|id| Entry::from_id(id)).collect();
        let mut rest: Vec<Entry> = self.home_rest.iter().filter_map(|id| Entry::from_id(id)).collect();
        // 每条都在某处、不重复：和设置文件读进来时同一套规矩。
        normalize(&mut sections, &mut top, &mut rest);
        state.home_rest_at = self.home_rest_at.filter(|at| *at <= sections.len());
        state.home_sections = sections;
        state.home_top = top;
        state.home_rest = rest;
        let took = from_ids(self.tiles.iter().map(String::as_str));
        if !took.is_empty() {
            state.tiles = took;
        }
    }

    /// 写进文件的样子：`kind` 与版本号总是补上。
    pub fn to_json(&self) -> String {
        let mut me = self.clone();
        me.kind = KIND.to_string();
        me.version = VERSION;
        serde_json::to_string_pretty(&me).expect("a layout always makes JSON") + "\n"
    }

    /// 从文件读回来。`kind` 空着的（手写的）也认，写着别的就不认。
    pub fn from_json(text: &str) -> Result<Layout> {
        let layout: Layout = serde_json::from_str(text).context("read the layout")?;
        if !layout.kind.is_empty() && layout.kind != KIND {
            bail!("this file is a {} and not a layout", layout.kind);
        }
        Ok(layout)
    }
}

/// 存储里的一个布局文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// 打开它用的整条路径。
    pub path: String,
    /// 文件名，页面上就写这个。
    pub name: String,
    /// 多少字节。
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_survives_the_trip_through_a_file() {
        let mut before = State::default();
        before.home_title = "槑的小天地".into();
        before.entry_names.insert("files".into(), "我的文件".into());
        before.home_sections = vec![Section {
            name: "常用".into(),
            entries: vec![Entry::Files],
            icon: crate::icons::Icon::Folder,
            collapsed: false,
        }];
        before.home_rest.retain(|entry| *entry != Entry::Files);
        before.home_rest_at = Some(0);
        before.home_rest_collapsed = true;
        before.tiles = vec![crate::app::Tile::Wlan, crate::app::Tile::Charge];

        let text = Layout::of(&before).to_json();
        assert!(text.contains("skyward-layout"), "文件里写着它是什么");

        let mut after = State::default();
        Layout::from_json(&text).expect("reads back").apply(&mut after);
        assert_eq!(after.home_title, "槑的小天地");
        assert_eq!(after.entry_names.get("files").map(String::as_str), Some("我的文件"));
        assert_eq!(after.home_sections.len(), 1);
        assert_eq!(after.home_sections[0].name, "常用");
        assert_eq!(after.home_sections[0].entries, vec![Entry::Files]);
        assert_eq!(after.home_rest_at, Some(0));
        assert!(after.home_rest_collapsed);
        assert_eq!(after.tiles, vec![crate::app::Tile::Wlan, crate::app::Tile::Charge]);
        assert!(!after.home_rest.contains(&Entry::Files), "进了分类就不在「其他」里");
        assert_eq!(Layout::of(&after), Layout::of(&before), "再导出一次是同一份");
    }

    #[test]
    fn another_file_is_turned_down() {
        let err = Layout::from_json("{\"kind\": \"something-else\"}").expect_err("not a layout");
        assert!(format!("{err:#}").contains("something-else"));
        assert!(Layout::from_json("not json at all").is_err());
        // 手写的、没有 kind 的也认。
        assert!(Layout::from_json("{\"home_title\": \"槑\"}").is_ok());
    }

    #[test]
    fn names_this_version_does_not_know_are_left_out() {
        let text = "{\"kind\":\"skyward-layout\",\"home_sections\":[{\"name\":\"甲\",\"entries\":[\"files\",\"no-such-entry\"]}],\
                    \"tiles\":[\"wlan\",\"advanced\",\"charge\"]}";
        let mut state = State::default();
        Layout::from_json(text).expect("reads").apply(&mut state);
        assert_eq!(state.home_sections[0].entries, vec![Entry::Files], "认不出的条目丢掉");
        assert_eq!(state.tiles, vec![crate::app::Tile::Wlan, crate::app::Tile::Charge], "去掉的那枚磁贴也丢掉");
    }
}
