#!/system/bin/sh
echo "id: $(id)"
for p in /proc/[0-9]*; do
  pid=${p#/proc/}
  comm=$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null)
  for f in cwd root exe; do
    t=$(readlink "$p/$f" 2>/dev/null)
    case "$t" in *system_ext*) echo "$f $pid -> $t : $comm";; esac
  done
  ls -l "$p/fd" 2>/dev/null | grep system_ext | sed "s|^|fd $pid : $comm |"
  grep -m2 system_ext "$p/maps" 2>/dev/null | sed "s|^|map $pid : $comm |"
  ls -l "$p/map_files" 2>/dev/null | grep system_ext | sed "s|^|mapf $pid : $comm |"
done
echo "=== namespaces ==="
ls -l /proc/[0-9]*/ns/mnt 2>/dev/null
echo "=== holders ==="
cat /proc/1/mountinfo | grep system_ext
echo "=== done ==="
