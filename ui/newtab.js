const cfg = window.__STRATA || { template: 'https://duckduckgo.com/?q=%s', engine: '' };
document.getElementById('name').textContent = cfg.name || '';
const q = document.getElementById('q');
q.placeholder = cfg.engine ? 'Search with ' + cfg.engine + ' or enter address' : 'Search or enter address';
document.getElementById('f').onsubmit = (e) => {
  e.preventDefault();
  const v = q.value.trim();
  if (!v) return;
  location.href = /^[\w-]+(\.[\w-]+)+(:\d+)?(\/\S*)?$/.test(v) ? 'https://' + v
    : /^https?:\/\/\S+$/.test(v) ? v : cfg.template.replace('%s', encodeURIComponent(v));
};
const sites = [['Wikipedia', 'https://www.wikipedia.org/'], ['OpenStreetMap', 'https://www.openstreetmap.org/'],
  ['Debian', 'https://www.debian.org/'], ['Rust', 'https://www.rust-lang.org/'], ['GitHub', 'https://github.com/']];
const nav = document.getElementById('sites');
for (const [n, u] of sites) {
  const a = document.createElement('a');
  a.href = u;
  const b = document.createElement('b');
  b.textContent = n[0];
  b.style.cssText = `background:hsl(${[...n].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)} 55% 45%);color:#fff;border:0`;
  a.append(b, n);
  nav.append(a);
}
