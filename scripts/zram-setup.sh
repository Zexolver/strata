#!/bin/sh
# One-time, needs root. Creates compressed swap in RAM: pages go in with lzo-rle (fast), and pages
# untouched for 3 minutes are recompressed with zstd (smaller). Needs kernel >= 6.1 (zram recompress).
# usage: sudo scripts/zram-setup.sh [size, default 50% of RAM, e.g. 4G]
set -eu
[ "$(id -u)" = 0 ] || { echo "run as root"; exit 1; }
modprobe zram num_devices=1
Z=/sys/block/zram0
[ -e $Z/recomp_algorithm ] || { echo "kernel too old: no zram recompress (need 6.1+ with CONFIG_ZRAM_MULTI_COMP)"; exit 1; }
swapoff /dev/zram0 2>/dev/null || true
echo 1 > $Z/reset
echo lzo-rle > $Z/comp_algorithm
echo "algo=zstd priority=1" > $Z/recomp_algorithm
echo "${1:-$(awk '/MemTotal/{print int($2/2)"K"}' /proc/meminfo)}" > $Z/disksize
mkswap /dev/zram0 >/dev/null
swapon -p 100 /dev/zram0
cat > /usr/local/sbin/zram-tier <<'T'
#!/bin/sh
while sleep 20; do
  echo 180 > /sys/block/zram0/idle 2>/dev/null && echo "type=idle algo=zstd" > /sys/block/zram0/recompress 2>/dev/null
done
T
chmod +x /usr/local/sbin/zram-tier
cat > /etc/systemd/system/zram-tier.service <<'T'
[Unit]
Description=Recompress cold zram pages with zstd
After=swap.target
[Service]
ExecStart=/usr/local/sbin/zram-tier
[Install]
WantedBy=multi-user.target
T
systemctl daemon-reload 2>/dev/null && systemctl enable --now zram-tier.service 2>/dev/null || /usr/local/sbin/zram-tier &
swapon --show
echo "Not persistent across reboots for the swap device itself; add this script to boot if you want it permanent."
