const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

let state = { settings: {}, profiles: [], tabs: [], active: null };
const info = new Map(); // tab id -> { url, title, loading }

const isNewTab = (u) => !u;
const profileOf = (id) => state.profiles.find((p) => p.id === id);

function render() {
  const s = state.settings;
  document.title = state.app_name || '';
  document.documentElement.dataset.theme = s.theme === 'system' ? '' : s.theme;
  const multi = s.profiles_enabled;

  const tabs = $('tabs');
  tabs.replaceChildren();
  for (const t of state.tabs) {
    const i = info.get(t.id) || {};
    const el = document.createElement('div');
    el.className = 'tab' + (t.id === state.active ? ' active' : '') + (t.discarded ? ' discarded' : '');
    if (multi) el.style.borderTopColor = profileOf(t.profile)?.color || 'transparent';
    const title = document.createElement('span');
    title.className = 'title';
    title.textContent = isNewTab(i.url) ? (i.title || 'New tab') : i.title || (i.loading ? 'Loading…' : i.url);
    if (i.loading) { const sp = document.createElement('span'); sp.className = 'spin'; el.append(sp); }
    const x = document.createElement('button');
    x.className = 'x';
    x.textContent = '×';
    x.onclick = (e) => { e.stopPropagation(); invoke('close_tab', { id: t.id }); };
    el.onclick = () => { invoke('switch_tab', { id: t.id }); };
    const ico = document.createElement('img');
    ico.className = 'ico' + (i.icon ? '' : ' hidden');
    if (i.icon) ico.src = i.icon;
    el.append(ico, title, x);
    if (i.loading) el.querySelector('.ico').replaceWith(el.querySelector('.spin'));
    tabs.append(el);
  }

  const cur = info.get(state.active);
  if (document.activeElement !== $('url')) $('url').value = isNewTab(cur?.url) ? '' : cur.url;

  const sel = $('newprofile');
  sel.classList.toggle('hidden', !multi);
  const prev = sel.value;
  sel.replaceChildren(...state.profiles.map((p) => new Option(p.name, p.id)));
  if (state.profiles.some((p) => p.id === prev)) sel.value = prev;
}

const active = () => state.active;
$('add').onclick = () => { invoke('new_tab', { profile: $('newprofile').value || null }); $('url').focus(); };
$('back').onclick = () => invoke('history_back', { id: active() });
$('fwd').onclick = () => invoke('history_forward', { id: active() });
$('reload').onclick = () => invoke('reload', { id: active() });
$('gear').onclick = () => invoke('open_settings');
$('url').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') { invoke('navigate', { id: active(), input: $('url').value }); $('url').blur(); }
});
$('url').addEventListener('focus', () => $('url').select());
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
    icon: u.url && u.url !== cur.url ? u.icon ?? null : u.icon ?? cur.icon,
  });
  render();
});
invoke('get_state').then((s) => { state = s; render(); });
