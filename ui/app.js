const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

let state = { settings: {}, profiles: [], tabs: [], active: null };
const info = new Map(); // tab id -> { url, title, loading }
let panelOpen = false;

const isNewTab = (u) => !u;
const profileOf = (id) => state.profiles.find((p) => p.id === id);

function render() {
  const s = state.settings;
  document.documentElement.dataset.theme = s.theme === 'system' ? '' : s.theme;
  const multi = s.profiles_enabled;

  const tabs = $('tabs');
  tabs.replaceChildren();
  for (const t of state.tabs) {
    const i = info.get(t.id) || {};
    const el = document.createElement('div');
    el.className = 'tab' + (t.id === state.active && !panelOpen ? ' active' : '');
    if (multi) el.style.borderTopColor = profileOf(t.profile)?.color || 'transparent';
    const title = document.createElement('span');
    title.className = 'title';
    title.textContent = isNewTab(i.url) ? (i.title || 'New tab') : i.title || (i.loading ? 'Loading…' : i.url);
    if (i.loading) { const sp = document.createElement('span'); sp.className = 'spin'; el.append(sp); }
    const x = document.createElement('button');
    x.className = 'x';
    x.textContent = '×';
    x.onclick = (e) => { e.stopPropagation(); invoke('close_tab', { id: t.id }); };
    el.onclick = () => { setPanel(false); invoke('switch_tab', { id: t.id }); };
    el.append(title, x);
    if (i.loading) el.prepend(el.querySelector('.spin'));
    tabs.append(el);
  }

  const cur = info.get(state.active);
  if (document.activeElement !== $('url')) $('url').value = isNewTab(cur?.url) ? '' : cur.url;

  const sel = $('newprofile');
  sel.classList.toggle('hidden', !multi);
  const prev = sel.value;
  sel.replaceChildren(...state.profiles.map((p) => new Option(p.name, p.id)));
  if (state.profiles.some((p) => p.id === prev)) sel.value = prev;

  const eng = $('engine');
  eng.replaceChildren(...state.engines.map(([id, name]) => new Option(name, id)), new Option('Custom…', 'custom'));
  eng.value = s.search_engine;
  $('customrow').classList.toggle('hidden', s.search_engine !== 'custom');
  if (document.activeElement !== $('customsearch')) $('customsearch').value = s.custom_search;
  $('theme').value = s.theme;
  $('adblock').checked = !!s.adblock;
  $('profiles').checked = !!multi;
  $('profilebox').classList.toggle('hidden', !multi);
  const list = $('profilelist');
  list.replaceChildren();
  for (const p of state.profiles) {
    const row = document.createElement('div');
    row.className = 'row';
    const name = document.createElement('span');
    name.textContent = p.name;
    name.style.borderLeft = `4px solid ${p.color}`;
    name.style.paddingLeft = '8px';
    if (p.proxy) { const b = document.createElement('span'); b.className = 'badge'; b.textContent = p.proxy; name.append(b); }
    row.append(name);
    if (p.id !== 'default') {
      const del = document.createElement('button');
      del.textContent = 'Delete';
      del.onclick = () => confirm(`Delete "${p.name}" and all its cookies and data?`) && invoke('remove_profile', { id: p.id }).catch(showErr);
      row.append(del);
    }
    list.append(row);
  }
}

function showErr(e) { $('err').textContent = String(e); }

function setPanel(on) {
  panelOpen = on;
  $('panel').classList.toggle('hidden', !on);
  invoke('set_overlay', { on });
  render();
}

const active = () => state.active;
$('add').onclick = () => { setPanel(false); invoke('new_tab', { profile: $('newprofile').value || null }); $('url').focus(); };
$('back').onclick = () => invoke('history_back', { id: active() });
$('fwd').onclick = () => invoke('history_forward', { id: active() });
$('reload').onclick = () => invoke('reload', { id: active() });
$('gear').onclick = () => setPanel(!panelOpen);
$('url').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') { setPanel(false); invoke('navigate', { id: active(), input: $('url').value }); $('url').blur(); }
});
$('url').addEventListener('focus', () => $('url').select());
const patch = (p) => invoke('update_settings', { patch: p }).catch((e) => ($('err2').textContent = String(e)));
$('theme').onchange = (e) => patch({ theme: e.target.value });
$('adblock').onchange = (e) => patch({ adblock: e.target.checked });
$('profiles').onchange = (e) => patch({ profiles_enabled: e.target.checked });
$('engine').onchange = (e) => patch({ search_engine: e.target.value });
$('customsearch').onchange = (e) => patch({ custom_search: e.target.value });
$('padd').onclick = () => {
  $('err').textContent = '';
  invoke('add_profile', { name: $('pname').value, color: $('pcolor').value, proxy: $('pproxy').value || null })
    .then(() => { $('pname').value = ''; $('pproxy').value = ''; })
    .catch(showErr);
};
document.addEventListener('keydown', (e) => {
  if (!(e.ctrlKey || e.metaKey)) return;
  if (e.key === 'l') { e.preventDefault(); $('url').focus(); }
  if (e.key === 't') { e.preventDefault(); $('add').click(); }
  if (e.key === 'w') { e.preventDefault(); invoke('close_tab', { id: active() }); }
});

listen('state', (e) => { state = e.payload; render(); });
listen('tab-update', (e) => {
  const u = e.payload;
  if (u.url && !/^https?:/.test(u.url)) u.url = null;
  const cur = info.get(u.id) || {};
  info.set(u.id, {
    url: u.url ?? cur.url,
    title: u.title ?? cur.title,
    loading: u.loading ?? cur.loading,
  });
  render();
});
invoke('get_state').then((s) => { state = s; render(); });
