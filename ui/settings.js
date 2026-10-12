const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);
const SECTIONS = [['appearance', 'Appearance'], ['search', 'Search'], ['privacy', 'Privacy & security'], ['profiles', 'Profiles']];
let state;

const hue = (s) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
function mono(name) {
  const m = document.createElement('span');
  m.className = 'mono';
  m.textContent = name[0].toUpperCase();
  m.style.background = `hsl(${hue(name)} 55% 45%)`;
  return m;
}
const fail = (e) => ($('err').textContent = String(e));
const patch = (p) => invoke('update_settings', { patch: p }).then(refresh).catch(fail);

function show(id) {
  for (const [s] of SECTIONS) $(s).classList.toggle('hidden', s !== id);
  for (const b of $('side').children) b.classList.toggle('on', b.dataset.id === id);
  location.hash = id;
}

function render() {
  const s = state.settings;
  document.documentElement.dataset.theme = s.theme === 'system' ? '' : s.theme;
  $('theme').value = s.theme;
  $('adblock').checked = !!s.adblock;
  $('profilesOn').checked = !!s.profiles_enabled;
  $('profilebox').classList.toggle('hidden', !s.profiles_enabled);
  $('customrow').classList.toggle('hidden', s.search_engine !== 'custom');
  if (document.activeElement !== $('customsearch')) $('customsearch').value = s.custom_search;

  $('engines').replaceChildren(...[...state.engines, ['custom', 'Custom…']].map(([id, name]) => {
    const l = document.createElement('label');
    l.className = 'eng';
    const r = document.createElement('input');
    r.type = 'radio';
    r.name = 'engine';
    r.checked = s.search_engine === id;
    r.onchange = () => patch({ search_engine: id });
    l.append(r, mono(name), name);
    return l;
  }));

  const list = $('profilelist');
  list.replaceChildren();
  for (const p of state.profiles) {
    const row = document.createElement('div');
    row.className = 'row';
    const name = document.createElement('span');
    name.textContent = p.name;
    name.style.cssText = `border-left:4px solid ${p.color};padding-left:8px`;
    if (p.proxy) { const b = document.createElement('span'); b.className = 'badge'; b.textContent = p.proxy; name.append(b); }
    row.append(name);
    if (p.id !== 'default') {
      const del = document.createElement('button');
      del.textContent = 'Delete';
      del.onclick = () => confirm(`Delete "${p.name}" and all its cookies and data?`) &&
        invoke('remove_profile', { id: p.id }).then(refresh).catch(fail);
      row.append(del);
    }
    list.append(row);
  }
}

function refresh() {
  $('err').textContent = '';
  return invoke('get_state').then((s) => { state = s; render(); });
}

for (const [id, label] of SECTIONS) {
  const b = document.createElement('button');
  b.textContent = label;
  b.dataset.id = id;
  b.onclick = () => show(id);
  $('side').append(b);
}
$('theme').onchange = (e) => patch({ theme: e.target.value });
$('adblock').onchange = (e) => patch({ adblock: e.target.checked });
$('profilesOn').onchange = (e) => patch({ profiles_enabled: e.target.checked });
$('customsearch').onchange = (e) => patch({ custom_search: e.target.value });
$('padd').onclick = () =>
  invoke('add_profile', { name: $('pname').value, color: $('pcolor').value, proxy: $('pproxy').value || null })
    .then(() => { $('pname').value = ''; $('pproxy').value = ''; return refresh(); }).catch(fail);
show(SECTIONS.some(([s]) => '#' + s === location.hash) ? location.hash.slice(1) : 'appearance');
refresh();
