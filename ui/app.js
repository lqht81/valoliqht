"use strict";

const POLL_MS = 3000;
const FRIENDS_MS = 10000;
const ACCOUNT_REFRESH_MS = 10 * 60 * 1000;
const ALL = "all";

// Shown first in the weapon bar, in this order.
const FAVORITES = [
  "9c82e19d-4575-0200-1a81-3eacf00cf872", // Vandal
  "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a", // Phantom
  "a03b24d3-4319-996d-0f8c-94bbfba1dfc7", // Operator
  "e336c6b8-418d-9340-d77f-7a9e4cfe0702", // Sheriff
  "462080d1-4035-2937-7c09-27aa2a5c27a7", // Spectre
  "1baa85b4-4c70-1284-64bb-6481dfc3bb4e", // Ghost
  "29a0cfab-485b-f5d5-779a-b59f85e204a8", // Classic
  "2f59173c-4bed-b6c3-2191-dea9b58be9c7", // Melee
];

const STATUS_TEXT = {
  riot_closed: "Riot kapalı",
  waiting: "Bekleniyor",
  client_only: "Oyun kapalı",
  menu: "Menüde",
  pregame: "Ajan seçimi",
  ingame: "Maçta",
  error: "Hata",
};

const EMPTY_TITLE = {
  riot_closed: "Riot Client açık değil",
  waiting: "VALORANT bekleniyor",
  error: "Bir sorun oluştu",
};

const $ = (id) => document.getElementById(id);

const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* ignore */ } },
};

// ---------- backend ----------

const tauri = window.__TAURI__;
let demo = null;

async function call(cmd, args) {
  if (tauri) return tauri.core.invoke(cmd, args);
  if (!demo) demo = (await import("./demo.js")).default;
  await new Promise((r) => setTimeout(r, 120));
  const map = {
    snapshot: () => (location.search.includes("lobby") ? demo.lobby : demo.snapshot),
    player_stats: () => demo.stats,
    match_detail: () => demo.scoreboard,
    friends: () => demo.friends,
    account: () => demo.account,
    inventory: () => demo.inventory,
    catalog: () => demo.catalog,
    check_update: () => demo.update,
    app_version: () => "demo",
    set_advanced: () => null,
    apply_update: () => null,
  };
  if (!map[cmd]) throw new Error("unknown command " + cmd);
  return map[cmd]();
}

// ---------- helpers ----------

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

function img(src, cls = "", alt = "", title = "") {
  return src
    ? `<img class="${cls}" src="${esc(src)}" alt="${esc(alt)}"${title ? ` title="${esc(title)}"` : ""} loading="lazy" draggable="false">`
    : "";
}

function tierStyle(color) {
  return color ? ` style="--tier:${esc(color)}"` : "";
}

function tierName(name) {
  // valorant-api gives upper-case names; show them in title case.
  return String(name || "").toLocaleLowerCase("tr-TR").replace(/(^|\s)(\S)/g, (m, a, b) => a + b.toLocaleUpperCase("tr-TR"));
}

function duration(sec) {
  if (sec == null || sec < 0) return "";
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d > 0) return `${d} gün ${h} sa`;
  if (h > 0) return `${h} sa ${m} dk`;
  return `${m} dk`;
}

function untilIso(iso) {
  const t = Date.parse(iso || "");
  return Number.isFinite(t) ? duration((t - Date.now()) / 1000) : "";
}

function num(n) {
  return n == null ? "—" : Number(n).toLocaleString("tr-TR");
}

function levelBadge(level, border) {
  if (level == null) return "";
  const bg = border ? ` style="background-image:url('${esc(border)}')"` : "";
  return `<span class="lvl${border ? " framed" : ""}"${bg} title="Hesap seviyesi">${level}</span>`;
}

// ---------- toasts ----------

function toast(id, html, sticky = false) {
  const box = $("toasts");
  let el = id ? box.querySelector(`[data-id="${id}"]`) : null;
  if (!el) {
    el = document.createElement("div");
    el.className = "toast";
    if (id) el.dataset.id = id;
    box.appendChild(el);
  }
  el.innerHTML = `${html}<button class="x" aria-label="Kapat">×</button>`;
  if (!sticky) setTimeout(() => el.remove(), 12000);
  return el;
}

$("toasts").addEventListener("click", (e) => {
  if (e.target.closest(".x")) e.target.closest(".toast").remove();
});

// ---------- wish list ----------

let wish = new Set(JSON.parse(store.get("wishlist") || "[]"));
let catalog = null;

function saveWish() {
  store.set("wishlist", JSON.stringify([...wish]));
}

function star(id) {
  if (!id) return "";
  const on = wish.has(id);
  return `<button class="star${on ? " on" : ""}" data-wish="${esc(id)}" title="${on ? "İstek listesinden çıkar" : "İstek listesine ekle"}" aria-pressed="${on}">★</button>`;
}

document.addEventListener("click", (e) => {
  const b = e.target.closest("[data-wish]");
  if (!b) return;
  e.stopPropagation();
  const id = b.dataset.wish;
  if (wish.has(id)) wish.delete(id);
  else wish.add(id);
  saveWish();
  refreshAfterWish();
});

function refreshAfterWish() {
  if (accountData && tab === "account" && !$("account").hidden) renderAccount();
  if (last) renderMatch(last);
  checkWishHits();
}

let announced = new Set(JSON.parse(store.get("wish-announced") || "[]"));

function checkWishHits() {
  const st = accountData && accountData.store;
  if (!st) return;
  const offers = [...st.daily, ...st.night];
  const hits = offers.filter((o) => o.skin_id && wish.has(o.skin_id));
  const today = new Date().toISOString().slice(0, 10);
  for (const h of hits) {
    const key = `${today}:${h.skin_id}`;
    if (announced.has(key)) continue;
    announced.add(key);
    toast(`wish-${h.skin_id}`, `<b>İstek listendeki skin mağazanda!</b><span>${esc(h.name)} · ${num(h.price)} VP</span>`, true);
  }
  store.set("wish-announced", JSON.stringify([...announced].slice(-50)));
}

// ---------- advanced switch ----------

let advanced = store.get("advanced") === "1";

function paintSwitch() {
  $("advanced").setAttribute("aria-checked", String(advanced));
}

async function setAdvanced(on) {
  advanced = on;
  store.set("advanced", on ? "1" : "0");
  paintSwitch();
  if (!on) {
    for (const p of [...open]) if (!last || !last.players.some((x) => x.puuid === p && x.is_me)) open.delete(p);
  }
  try { await call("set_advanced", { on }); } catch { /* ignore */ }
  lastKey = "";
  rerender();
}

$("advanced").addEventListener("click", () => {
  if (advanced) setAdvanced(false);
  else {
    $("risk").hidden = false;
    $("risk-cancel").focus();
  }
});
$("risk-cancel").addEventListener("click", () => { $("risk").hidden = true; });
$("risk-ok").addEventListener("click", () => {
  $("risk").hidden = true;
  setAdvanced(true);
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("risk").hidden) $("risk").hidden = true;
});

// ---------- tabs ----------

let tab = "match";

function setTab(next) {
  tab = next;
  for (const b of document.querySelectorAll(".tab")) {
    b.setAttribute("aria-selected", String(b.dataset.tab === next));
  }
  $("view-match").hidden = next !== "match";
  $("view-friends").hidden = next !== "friends";
  $("view-account").hidden = next !== "account";
  $("match").style.visibility = next === "match" ? "visible" : "hidden";
  if (next === "friends") loadFriends();
  if (next === "account") {
    showInventory(false);
    loadAccount(false);
  }
}

$("tabs").addEventListener("click", (e) => {
  const b = e.target.closest(".tab");
  if (b) setTab(b.dataset.tab);
});

// ---------- shared blocks ----------

const statsCache = new Map(); // puuid -> {state, data|error}
const boards = new Map(); // match id -> {state, data|error}
const openBoards = new Set();

function missionList(list, refill) {
  if (!list || !list.length) return `<div class="note">Görev bilgisi yok.</div>`;
  const kind = { daily: "Günlük", weekly: "Haftalık", other: "Görev" };
  const rows = list.map((m) => {
    const pct = m.target ? Math.min(100, Math.round((m.progress / m.target) * 100)) : 0;
    const left = m.expires ? untilIso(m.expires) : "";
    return `<div class="mission${m.complete ? " done" : ""}">
      <div class="m-head"><span class="m-kind ${esc(m.kind)}">${kind[m.kind] || "Görev"}</span>
        <span class="m-title" title="${esc(m.title)}">${esc(m.title)}</span>
        <span class="m-xp">${m.xp ? num(m.xp) + " XP" : ""}</span></div>
      <div class="bar"><i style="width:${pct}%"></i></div>
      <div class="m-foot"><span>${m.complete ? "Tamamlandı" : `${num(m.progress)} / ${num(m.target)}`}</span><span>${left ? left + " kaldı" : ""}</span></div>
    </div>`;
  }).join("");
  const refillText = refill && untilIso(refill) ? `<div class="m-refill">Haftalık görevler ${untilIso(refill)} sonra yenilenir</div>` : "";
  return `<div class="missions">${rows}</div>${refillText}`;
}

function offerCard(o, compact = false) {
  const price = o.discount
    ? `<b>${num(o.price)} VP</b><s>${num(o.original)}</s><span class="off">-%${o.discount}</span>`
    : `<b>${num(o.price)} VP</b>`;
  const wished = o.skin_id && wish.has(o.skin_id);
  return `<div class="offer${compact ? " compact" : ""}${wished ? " wished" : ""}"${tierStyle(o.tier_color)}>
    ${star(o.skin_id)}
    ${img(o.image, "", o.name)}
    <div class="o-name" title="${esc(o.name)}">${esc(o.name)}</div>
    <div class="price">${price}</div>
  </div>`;
}

function scoreRows(rows) {
  return rows.map((r) => `<div class="sb-row${r.is_me ? " me" : ""}">
    ${img(r.agent_icon, "", r.agent || "", r.agent || "")}
    <span class="sb-name">${r.name ? `${esc(r.name)}<span class="tag">#${esc(r.tag || "")}</span>` : `<i>${esc(r.agent || "Oyuncu")}</i>`}</span>
    <span class="sb-tier">${r.tier ? img(r.tier.icon, "", r.tier.name, tierName(r.tier.name)) : ""}</span>
    <span>${r.kills}/${r.deaths}/${r.assists}</span>
    <span>${r.acs}</span><span>${r.adr}</span><span>${r.hs != null ? r.hs + "%" : "—"}</span>
  </div>`).join("");
}

function scoreboard(id) {
  const b = boards.get(id);
  if (!b || b.state === "loading") return `<div class="board"><div class="loading">Skor tablosu yükleniyor…</div></div>`;
  if (b.state === "error") return `<div class="board"><div class="err">${esc(b.error)}</div></div>`;
  const s = b.data;
  const head = `<div class="sb-row sb-head"><span></span><span>Oyuncu</span><span></span><span>K/Ö/A</span><span>ACS</span><span>ADR</span><span>HS</span></div>`;
  return `<div class="board">
    <div class="sb-title">${esc(s.map || "?")} · ${esc(s.queue)} · ${s.rounds_us}-${s.rounds_them}${s.minutes ? ` · ${s.minutes} dk` : ""}</div>
    <div class="sb-team ally"><div class="sb-label">Takımın</div>${head}${scoreRows(s.mine)}</div>
    <div class="sb-team enemy"><div class="sb-label">Rakipler</div>${head}${scoreRows(s.theirs)}</div>
    <div class="sb-note">Senin, partindekilerin ve arkadaşlarının dışındakilerin adı gösterilmez.</div>
  </div>`;
}

function statsBlock(puuid, own = false) {
  const st = statsCache.get(puuid);
  if (!st || st.state === "loading") return `<div class="stats"><div class="loading">Son maçlar yükleniyor…</div></div>`;
  if (st.state === "error") return `<div class="stats"><div class="err">${esc(st.error)}</div></div>`;
  const s = st.data;
  if (!s.matches.length) return `<div class="stats"><div class="err">Yakın zamanda oynanmış maç bulunamadı.</div></div>`;
  const chip = (label, value) => `<span class="stat">${label}<b>${value}</b></span>`;
  const mains = s.agents.map((a) => img(a.icon, "", a.name, `${a.name} · ${a.count} maç`)).join("");
  const games = s.matches.map((m) => {
    const cls = m.won == null ? "" : m.won ? " win" : " loss";
    const res = m.won == null ? "—" : `${m.rounds_us}-${m.rounds_them}`;
    const openNow = own && openBoards.has(m.match_id);
    return `<div class="game${cls}${own ? " clickable" : ""}"${own ? ` data-match="${esc(m.match_id)}" title="Skor tablosunu aç"` : ""}>${img(m.agent_icon, "", m.agent || "")}
      <span class="map">${esc(m.map || "?")} · ${esc(m.queue)}</span>
      <span class="res">${res}</span>
      <span class="kda">${m.kills}/${m.deaths}/${m.assists}</span>
      <span class="acs">${m.acs} ACS</span></div>${openNow ? scoreboard(m.match_id) : ""}`;
  }).join("");
  return `<div class="stats">
    <div class="stat-row">
      ${chip("K/D", s.kd ?? "—")}${chip("ACS", s.acs ?? "—")}${chip("HS", s.hs != null ? s.hs + "%" : "—")}${chip("Kazanma", s.winrate != null ? s.winrate + "%" : "—")}
      <span class="mains">${mains}</span>
    </div>
    <div class="games">${games}</div>
  </div>`;
}

async function toggleBoard(id) {
  if (openBoards.has(id)) {
    openBoards.delete(id);
    renderAccount();
    return;
  }
  openBoards.add(id);
  if (!boards.has(id) || boards.get(id).state === "error") {
    boards.set(id, { state: "loading" });
    renderAccount();
    try {
      boards.set(id, { state: "done", data: await call("match_detail", { id }) });
    } catch (err) {
      boards.set(id, { state: "error", error: String(err) });
    }
  }
  renderAccount();
}

// ---------- match ----------

let selected = store.get("weapon") || FAVORITES[0];
let last = null;
let lastKey = "";
const open = new Set();

function orderedWeapons(list) {
  const rank = (w) => {
    const i = FAVORITES.indexOf(w.id);
    return i === -1 ? 100 : i;
  };
  return [...list].sort((a, b) => rank(a) - rank(b));
}

function renderWeapons(snap) {
  const nav = $("weapons");
  if (!snap.players.length || !snap.weapons.length) {
    nav.innerHTML = "";
    return;
  }
  const weapons = orderedWeapons(snap.weapons);
  if (selected !== ALL && !weapons.some((w) => w.id === selected)) {
    selected = weapons[0].id;
  }
  const chip = (id, label, icon) =>
    `<button class="chip" data-id="${esc(id)}" aria-pressed="${id === selected}">${img(icon)}${esc(label)}</button>`;
  nav.innerHTML = chip(ALL, "Tümü", null) + weapons.map((w) => chip(w.id, w.name, w.icon)).join("");
}

function playerName(p) {
  const you = p.is_me ? `<span class="pill you">SEN</span>` : "";
  const party = advanced && p.party ? `<span class="pill party" title="Birlikte sıraya girmişler">GRUP ${p.party}</span>` : "";
  if (p.hidden) {
    return `<div class="name hidden">Gizli oyuncu${party}</div>`;
  }
  const tag = p.tag ? `<span class="tag">#${esc(p.tag)}</span>` : "";
  return `<div class="name"><span>${esc(p.name || "Oyuncu")}${tag}</span>${you}${party}</div>`;
}

function rankLine(r) {
  if (!r) return `<div class="rank loading">Rank yükleniyor…</div>`;
  const rr = r.rr != null ? `<span class="rr-val">${r.rr} RR</span>` : "";
  const peak = advanced && r.peak && r.peak.tier > r.current.tier
    ? `<span class="peak" title="En yüksek rank">${img(r.peak.icon)}${esc(tierName(r.peak.name))}</span>`
    : "";
  return `<div class="rank">${img(r.current.icon)}<span class="tier"${r.current.color ? ` style="color:${esc(r.current.color)}"` : ""}>${esc(tierName(r.current.name))}</span>${rr}${peak}</div>`;
}

function encounterLine(e) {
  if (!advanced || !e) return "";
  const when = e.ago === 1 ? "Son maçında" : `${e.ago} maç önce`;
  const side = e.ally ? "takım arkadaşındı" : "rakibindi";
  const result = e.won == null ? "" : e.won ? " · kazandın" : " · kaybettin";
  const times = e.times > 1 ? ` · son maçlarında ${e.times} kez` : "";
  return `<div class="enc ${e.ally ? "ally" : "foe"}" title="Kendi son maçlarından">
    ${img(e.agent_icon, "", e.agent || "")}<span><b>${when} ${side}</b>${e.map ? ` · ${esc(e.map)}` : ""}${result}
    · ${esc(e.agent || "")} ${e.kills}/${e.deaths}/${e.assists}, ${e.acs} ACS${times}</span></div>`;
}

function sprayRow(list) {
  if (!list || !list.length) return "";
  return `<div class="sprays">${list.map((x) => img(x.icon, "", x.name, x.name)).join("")}</div>`;
}

function skinBlock(w) {
  if (!w) return `<div class="skin"><span class="none">Silah bilgisi yok</span></div>`;
  return `<div class="skin${w.standard ? " standard" : ""}"${tierStyle(w.tier_color)}>
    ${img(w.image, "gun", w.skin)}${img(w.buddy, "buddy")}
    <div class="skin-name" title="${esc(w.skin)}">${esc(w.skin)}</div>
  </div>`;
}

function tile(w) {
  return `<div class="tile${w.standard ? " standard" : ""}"${tierStyle(w.tier_color)} title="${esc(w.skin)}">
    ${img(w.image, "gun", w.skin)}${img(w.buddy, "buddy")}
    <div class="t-name">${esc(w.skin)}</div>
  </div>`;
}

function playerCard(p) {
  const sub = [p.title && `<span class="title">${esc(p.title)}</span>`, p.agent && esc(p.agent)].filter(Boolean).join(" · ");
  const right = selected === ALL ? "<div></div>" : skinBlock(p.weapons.find((w) => w.weapon_id === selected));
  const grid = selected === ALL
    ? `<div class="grid">${p.weapons.length ? p.weapons.map(tile).join("") : `<div class="t-name">Silah bilgisi yok</div>`}</div>`
    : "";
  const face = p.agent_icon ? img(p.agent_icon, "agent", p.agent || "") : `<div class="agent"></div>`;
  const clickable = advanced || p.is_me;
  return `<article class="player${p.is_me ? " me" : ""}${clickable ? "" : " static"}" data-puuid="${esc(p.puuid)}"${advanced && p.party ? ` data-party="${((p.party - 1) % 5) + 1}"` : ""}>
    <div class="row">
      <div class="face">${face}${img(p.card, "card")}</div>
      <div class="who">${playerName(p)}<div class="sub">${levelBadge(p.level, p.border)}${sub || "Ajan seçiliyor"}</div>${rankLine(p.rank)}${sprayRow(p.sprays)}</div>
      ${right}
    </div>
    ${encounterLine(p.encounter)}
    ${grid}
    ${open.has(p.puuid) ? statsBlock(p.puuid) : ""}
  </article>`;
}

function lobbyMember(m) {
  return `<div class="member${m.is_me ? " me" : ""}">
    ${m.card ? img(m.card, "m-card") : `<div class="m-card"></div>`}
    <div class="who">
      <div class="name"><span>${esc(m.name || "Oyuncu")}${m.tag ? `<span class="tag">#${esc(m.tag)}</span>` : ""}</span>
        ${m.is_me ? `<span class="pill you">SEN</span>` : ""}${m.owner ? `<span class="pill owner" title="Parti lideri">LİDER</span>` : ""}</div>
      <div class="sub">${levelBadge(m.level, m.border)}${m.title ? `<span class="title">${esc(m.title)}</span>` : ""}</div>
    </div>
    <div class="rank">${img(m.rank.icon)}<span class="tier"${m.rank.color ? ` style="color:${esc(m.rank.color)}"` : ""}>${esc(tierName(m.rank.name))}</span></div>
  </div>`;
}

function renderLobby(snap) {
  const box = $("lobby");
  const a = accountData;
  const l = snap.lobby;
  const head = snap.status === "client_only"
    ? `<div class="lobby-head"><h1>VALORANT kapalı</h1><p>${esc(snap.message || "")}</p></div>`
    : `<div class="lobby-head"><h1>${l && l.searching ? "Maç aranıyor…" : "Maç bekleniyor"}</h1>
       <p>Ajan seçimine girdiğinde takımının, maç başlayınca herkesin bilgileri burada görünür.</p></div>`;
  const party = l
    ? `<div class="section"><h2>Partin<span class="timer">${esc(l.queue)}${l.searching ? " · sıra aranıyor" : ""}</span></h2>
        <div class="members">${l.members.map(lobbyMember).join("")}</div></div>`
    : "";
  const missions = `<div class="section"><h2>Görevler</h2>${a ? missionList(a.missions, a.weekly_refill)
    : `<div class="note">Görevler ve mağaza, <b>Hesabım</b> sekmesini açtığında burada da görünür.</div>`}</div>`;
  const st = a && a.store;
  const shop = st
    ? `<div class="section"><h2>Bugünkü mağaza<span class="timer">${duration(st.daily_ends_in)} sonra yenilenir</span></h2>
        <div class="offers mini">${st.daily.map((o) => offerCard(o, true)).join("")}</div></div>`
    : "";
  box.innerHTML = `${head}<div class="lobby-grid"><div class="lobby-main">${party}${shop}</div><aside>${missions}</aside></div>`;
}

function renderMatch(snap) {
  $("status").dataset.state = snap.status;
  $("status-text").textContent = STATUS_TEXT[snap.status] || snap.status;

  const parts = [];
  if (snap.map) parts.push(`<b>${esc(snap.map)}</b>`);
  if (snap.queue && snap.players.length) parts.push(esc(snap.queue));
  let html = parts.join(" · ");
  if (snap.score) html += ` <span class="score"><span class="us">${snap.score[0]}</span> – <span class="them">${snap.score[1]}</span></span>`;
  $("match").innerHTML = html;

  const inMatch = (snap.status === "pregame" || snap.status === "ingame") && snap.players.length;
  const lobby = !inMatch && (snap.status === "menu" || snap.status === "client_only");
  $("empty").hidden = !!inMatch || lobby;
  $("lobby").hidden = !lobby;
  $("teams").hidden = !inMatch;
  renderWeapons(snap);

  if (lobby) {
    renderLobby(snap);
    return;
  }
  if (!inMatch) {
    $("empty-title").textContent = EMPTY_TITLE[snap.status] || "Bekleniyor";
    $("empty-text").textContent = snap.message || "";
    return;
  }

  const ally = snap.players.filter((p) => p.ally);
  const enemy = snap.players.filter((p) => !p.ally);
  $("ally").innerHTML = ally.map(playerCard).join("");
  $("enemy").innerHTML = enemy.length
    ? enemy.map(playerCard).join("")
    : `<div class="note">${snap.status === "pregame" ? "Rakipler maç başlayınca görünür." : "Rakip yok."}</div>`;
}

function showSnapshot(snap) {
  const key = JSON.stringify(snap);
  last = snap;
  if (key === lastKey) return;
  lastKey = key;
  renderMatch(snap);
}

function rerender() {
  if (last) renderMatch(last);
}

async function toggleStats(puuid) {
  if (open.has(puuid)) {
    open.delete(puuid);
    rerender();
    return;
  }
  open.add(puuid);
  const cached = statsCache.get(puuid);
  if (cached && cached.state !== "error") {
    rerender();
    return;
  }
  statsCache.set(puuid, { state: "loading" });
  rerender();
  try {
    statsCache.set(puuid, { state: "done", data: await call("player_stats", { puuid }) });
  } catch (err) {
    statsCache.set(puuid, { state: "error", error: String(err) });
  }
  rerender();
}

$("weapons").addEventListener("click", (e) => {
  const btn = e.target.closest(".chip");
  if (!btn) return;
  selected = btn.dataset.id;
  store.set("weapon", selected);
  rerender();
});

$("teams").addEventListener("click", (e) => {
  if (e.target.closest(".stats")) return;
  const card = e.target.closest(".player");
  if (card && !card.classList.contains("static")) toggleStats(card.dataset.puuid);
});

async function poll() {
  if (!document.hidden) {
    try {
      showSnapshot(await call("snapshot"));
    } catch (err) {
      showSnapshot({ status: "error", message: String(err), players: [], weapons: [] });
    }
  }
  setTimeout(poll, tauri ? POLL_MS : 60000);
}

// ---------- friends ----------

let friendsTimer = null;

function friendRow(f) {
  const where = [f.map, f.queue].filter(Boolean).map(esc).join(" · ");
  const score = f.score ? ` · ${f.score[0]}-${f.score[1]}` : "";
  const party = f.party_size && f.party_size > 1 ? ` · ${f.party_size} kişilik grup` : "";
  const detail = f.in_valorant ? `${where}${score}${party}` : "";
  const rank = f.rank ? `${img(f.rank.icon)}<span>${esc(tierName(f.rank.name))}</span>` : "";
  const level = f.level ? `<span>Sv ${f.level}</span>` : "";
  return `<div class="friend${f.online ? "" : " offline"}" data-state="${esc(f.status)}">
    <span class="dot"></span>
    <div style="min-width:0">
      <div class="name"><span>${esc(f.name)}<span class="tag">#${esc(f.tag)}</span></span></div>
      <div class="detail">${esc(f.status)}${detail ? " · " + detail : ""}${f.note ? ` · <i>${esc(f.note)}</i>` : ""}</div>
    </div>
    <div class="right">${rank}${level}</div>
  </div>`;
}

async function loadFriends() {
  clearTimeout(friendsTimer);
  if (tab !== "friends") return;
  const box = $("friends");
  if (!box.innerHTML) box.innerHTML = `<div class="note">Arkadaşlar yükleniyor…</div>`;
  try {
    const list = await call("friends");
    const online = list.filter((f) => f.online).length;
    box.innerHTML = `<div class="page-head"><h1>Arkadaşlar</h1><span class="muted">${online} çevrimiçi · ${list.length} kişi</span></div>
      <div class="list">${list.length ? list.map(friendRow).join("") : `<div class="note">Arkadaş listen boş.</div>`}</div>`;
  } catch (err) {
    box.innerHTML = `<div class="note">${esc(err)}</div>`;
  }
  if (tauri) friendsTimer = setTimeout(loadFriends, FRIENDS_MS);
}

// ---------- account ----------

let accountData = null;
let accountAt = 0;
let accountBusy = false;
let myStats = null;
let collectionFilter = "";
let wishFilter = "";

async function ensureAccount() {
  if (accountBusy || (accountData && Date.now() - accountAt < ACCOUNT_REFRESH_MS)) return;
  accountBusy = true;
  try {
    accountData = await call("account", { force: false });
    accountAt = Date.now();
    checkWishHits();
    if (last && (last.status === "menu" || last.status === "client_only")) renderMatch(last);
  } catch { /* shown on the account tab */ }
  accountBusy = false;
}

function ownedGrid(a) {
  const q = collectionFilter.toLocaleLowerCase("tr-TR");
  const items = a.collection.filter((s) => !q || `${s.name} ${s.weapon}`.toLocaleLowerCase("tr-TR").includes(q));
  if (!items.length) return `<div class="note">${a.collection.length ? "Eşleşen skin yok." : "Koleksiyon boş."}</div>`;
  return `<div class="owned">${items.map((s) => `<div class="own"${tierStyle(s.tier_color)} title="${esc(s.name)}">
    ${img(s.image, "", s.name)}<div class="o-name">${esc(s.name)}</div><div class="o-weapon">${esc(s.weapon)}</div></div>`).join("")}</div>`;
}

function wishSection() {
  const byId = new Map((catalog || []).map((s) => [s.id, s]));
  const list = [...wish].map((id) => byId.get(id)).filter(Boolean);
  const mine = list.length
    ? `<div class="wish-list">${list.map((s) => `<div class="own"${tierStyle(s.tier_color)}>${star(s.id)}${img(s.image, "", s.name)}
        <div class="o-name">${esc(s.name)}</div><div class="o-weapon">${esc(s.weapon)}</div></div>`).join("")}</div>`
    : `<div class="note">${wish.size && !catalog ? "Liste yükleniyor…" : "Listen boş. Aşağıdan skin arayıp ★ ile ekle; mağazana düştüğünde uygulama haber verir."}</div>`;
  const q = wishFilter.trim().toLocaleLowerCase("tr-TR");
  const results = q.length >= 2 && catalog
    ? catalog.filter((s) => `${s.name} ${s.weapon}`.toLocaleLowerCase("tr-TR").includes(q)).slice(0, 24)
    : [];
  const found = results.length
    ? `<div class="owned">${results.map((s) => `<div class="own"${tierStyle(s.tier_color)}>${star(s.id)}${img(s.image, "", s.name)}
        <div class="o-name">${esc(s.name)}</div><div class="o-weapon">${esc(s.weapon)}</div></div>`).join("")}</div>`
    : q.length >= 2 ? `<div class="note">Sonuç yok.</div>` : "";
  return `${mine}<div class="collection-head" style="margin-top:12px"><span class="muted">Skin ekle</span>
    <input class="search" id="wish-search" placeholder="En az 2 harf yaz" value="${esc(wishFilter)}"></div><div id="wish-results">${found}</div>`;
}

async function ensureCatalog() {
  if (catalog) return;
  try {
    catalog = await call("catalog");
    if (tab === "account" && !$("account").hidden && accountData) renderAccount();
  } catch { /* ignore */ }
}

function renderAccount() {
  const a = accountData;
  const box = $("account");
  if (!a) return;
  const r = a.rank;
  const rankBlock = r
    ? `<div class="big-rank">${img(r.current.icon)}<div class="tier"${r.current.color ? ` style="color:${esc(r.current.color)}"` : ""}>${esc(tierName(r.current.name))}</div>
        <div class="small">${r.rr != null ? r.rr + " RR" : ""}${r.leaderboard ? ` · #${r.leaderboard}` : ""}</div>
        ${r.peak ? `<div class="small">En yüksek: ${esc(tierName(r.peak.name))}</div>` : ""}</div>`
    : "";
  const w = a.wallet || {};
  const act = r && r.games ? `${r.wins ?? 0}G / ${r.games - (r.wins ?? 0)}M` : "—";
  const st = a.store;

  const rr = a.rr_history.length
    ? `<div class="rr-list">${a.rr_history.map((m) => `<div class="rr">${img(m.tier.icon)}<span>${esc(m.map || "?")}</span>
        <span class="after">${m.rr_after} RR</span>
        <span class="delta ${m.earned >= 0 ? "up" : "down"}">${m.earned >= 0 ? "+" : ""}${m.earned}</span></div>`).join("")}</div>`
    : `<div class="note">Bu sezon rekabetçi maç yok.</div>`;

  const contracts = a.contracts.length
    ? a.contracts.map((c) => {
        const pct = c.total ? Math.min(100, Math.round((c.level / c.total) * 100)) : 0;
        return `<div class="contract"><div class="c-head"><span>${esc(c.kind)}: ${esc(c.name)}</span><span>${c.level} / ${c.total}</span></div>
          <div class="bar"><i style="width:${pct}%"></i></div></div>`;
      }).join("")
    : `<div class="note">Aktif kontrat yok.</div>`;

  let recent = `<div class="note">Son maçlar yükleniyor…</div>`;
  if (myStats && myStats.state === "done") {
    statsCache.set(a.puuid, myStats);
    recent = statsBlock(a.puuid, true);
  } else if (myStats && myStats.state === "error") {
    recent = `<div class="note">${esc(myStats.error)}</div>`;
  }

  const scroll = box.closest(".scroll");
  const keep = scroll ? scroll.scrollTop : 0;
  const focusId = document.activeElement && document.activeElement.id;

  box.innerHTML = `
    <div class="page-head"><h1>Hesabım</h1>
      <button class="btn" id="open-inventory">Hesap İçeriği ve Değeri</button>
      <button class="btn" id="account-refresh">Yenile</button></div>
    <div class="hero">
      ${img(a.card_wide, "banner")}
      <div class="hero-in">
        ${a.card_small ? img(a.card_small, "avatar") : `<div class="avatar"></div>`}
        <div>
          <h1>${esc(a.name || "Hesabım")}${a.tag ? `<span class="tag">#${esc(a.tag)}</span>` : ""}</h1>
          <div class="meta">${levelBadge(a.level, a.border)}${a.title ? `<span class="title">${esc(a.title)}</span>` : ""}</div>
        </div>
        ${rankBlock}
      </div>
    </div>

    <div class="cards">
      <div class="kpi"><div class="label">Valorant Points</div><div class="value">${num(w.vp)}<small>VP</small></div></div>
      <div class="kpi"><div class="label">Radianite</div><div class="value">${num(w.rp)}<small>RP</small></div></div>
      <div class="kpi"><div class="label">Kingdom Kredisi</div><div class="value">${num(w.kc)}</div></div>
      <div class="kpi"><div class="label">Bu act</div><div class="value">${act}</div></div>
      <div class="kpi"><div class="label">Koleksiyon</div><div class="value">${a.collection.length}<small>skin</small></div></div>
    </div>

    <div class="two wide-left">
      <div>
        ${st ? `<div class="section"><h2>Günlük mağaza<span class="timer">${duration(st.daily_ends_in)} sonra yenilenir</span></h2>
          <div class="offers">${st.daily.map((o) => offerCard(o)).join("")}</div></div>` : ""}
      </div>
      <div class="section"><h2>Görevler</h2>${missionList(a.missions, a.weekly_refill)}</div>
    </div>

    ${st && st.night.length ? `<div class="section"><h2>Night Market<span class="timer">${duration(st.night_ends_in)} kaldı</span></h2>
      <div class="offers">${st.night.map((o) => offerCard(o)).join("")}</div></div>` : ""}

    ${st && st.bundles.length ? `<div class="section"><h2>Öne çıkan paket</h2>${st.bundles.map((b) => `<div class="bundle">${img(b.image)}<div>
      <div class="o-name" style="font:600 18px var(--head)">${esc(b.name)}</div>
      <div class="muted">${b.ends_in != null ? duration(b.ends_in) + " kaldı" : ""}</div></div></div>`).join("")}</div>` : ""}

    <div class="section"><h2>İstek listesi<span class="timer">${wish.size} skin</span></h2>${wishSection()}</div>

    <div class="two">
      <div class="section"><h2>RR geçmişi</h2>${rr}</div>
      <div class="section"><h2>Kontratlar</h2>${contracts}</div>
    </div>

    <div class="section"><h2>Son maçların<span class="timer">Skor tablosu için maça tıkla</span></h2>${recent}</div>

    <div class="section">
      <div class="collection-head"><h2>Skin koleksiyonu</h2>
        <input class="search" id="skin-search" placeholder="Skin ya da silah ara" value="${esc(collectionFilter)}">
      </div>
      <div id="owned">${ownedGrid(a)}</div>
    </div>

    ${a.problems.length ? `<div class="problems">Yüklenemeyenler: ${a.problems.map(esc).join(" · ")}</div>` : ""}
  `;
  if (scroll) scroll.scrollTop = keep;
  if (focusId && $(focusId)) {
    const el = $(focusId);
    el.focus();
    if (el.setSelectionRange) el.setSelectionRange(el.value.length, el.value.length);
  }
}

async function loadAccount(force) {
  const box = $("account");
  if (!accountData) box.innerHTML = `<div class="note">Hesap bilgileri yükleniyor…</div>`;
  ensureCatalog();
  try {
    accountData = await call("account", { force });
    accountAt = Date.now();
    renderAccount();
    checkWishHits();
  } catch (err) {
    box.innerHTML = `<div class="page-head"><h1>Hesabım</h1><button class="btn" id="account-refresh">Tekrar dene</button></div><div class="note">${esc(err)}</div>`;
    return;
  }
  if (!myStats || force) {
    myStats = { state: "loading" };
    try {
      myStats = { state: "done", data: await call("player_stats", { puuid: accountData.puuid }) };
    } catch (err) {
      myStats = { state: "error", error: String(err) };
    }
    if (tab === "account") renderAccount();
  }
}

$("account").addEventListener("click", (e) => {
  if (e.target.closest("#account-refresh")) return void loadAccount(true);
  if (e.target.closest("#open-inventory")) return void showInventory(true);
  if (e.target.closest(".board")) return;
  const g = e.target.closest(".game[data-match]");
  if (g) toggleBoard(g.dataset.match);
});

$("account").addEventListener("input", (e) => {
  if (!accountData) return;
  if (e.target.id === "skin-search") {
    collectionFilter = e.target.value;
    $("owned").innerHTML = ownedGrid(accountData);
  } else if (e.target.id === "wish-search") {
    wishFilter = e.target.value;
    renderAccount();
  }
});

// ---------- inventory ----------

let inventoryData = null;
let inventoryGroup = "skins";
let inventoryFilter = "";

function showInventory(on) {
  $("account").hidden = on;
  $("inventory").hidden = !on;
  if (on) loadInventory(false);
}

function renderInventory() {
  const box = $("inventory");
  const inv = inventoryData;
  const head = `<div class="page-head"><button class="btn" id="inv-back">← Hesabım</button><h1>Hesap İçeriği ve Değeri</h1>
    <button class="btn" id="inv-refresh">Yenile</button></div>`;
  if (!inv) {
    box.innerHTML = `${head}<div class="note">Envanter yükleniyor…</div>`;
    return;
  }
  const g = inv.groups.find((x) => x.key === inventoryGroup) || inv.groups[0];
  const q = inventoryFilter.toLocaleLowerCase("tr-TR");
  const items = g.items.filter((i) => !q || `${i.name} ${i.sub || ""}`.toLocaleLowerCase("tr-TR").includes(q));
  const chips = inv.groups.map((x) => `<button class="chip" data-group="${esc(x.key)}" aria-pressed="${x.key === g.key}">${esc(x.label)} <span class="count">${x.items.length}</span></button>`).join("");
  const grid = items.length
    ? `<div class="owned">${items.map((i) => `<div class="own${g.key === "titles" ? " title-only" : ""}"${tierStyle(i.tier_color)} title="${esc(i.name)}">
        ${img(i.image, "", i.name)}<div class="o-name">${esc(i.name)}</div>
        <div class="o-weapon">${esc(i.sub || "")}${i.price != null ? `<span class="o-price"${i.estimated ? ` title="Kademe fiyatından tahmin"` : ""}>${i.estimated ? "~" : ""}${num(i.price)} VP</span>` : ""}</div></div>`).join("")}</div>`
    : `<div class="note">Bu bölümde bir şey yok.</div>`;
  const focusId = document.activeElement && document.activeElement.id;
  box.innerHTML = `${head}
    <div class="cards">
      <div class="kpi accent"><div class="label">Tahmini toplam değer</div><div class="value">${num(inv.value)}<small>VP</small></div></div>
      <div class="kpi"><div class="label">Fiyatı bilinen eşya</div><div class="value">${num(inv.priced)}</div></div>
      <div class="kpi"><div class="label">Fiyatı olmayan</div><div class="value">${num(inv.unpriced)}<small>BP, paket, ödül</small></div></div>
      <div class="kpi"><div class="label">Açık ajan</div><div class="value">${num(inv.agents)}</div></div>
    </div>
    <div class="section">
      <div class="inv-tabs">${chips}</div>
      <div class="collection-head"><h2>${esc(g.label)}<span class="timer">${num(g.value)} VP · ${g.items.length} adet</span></h2>
        <input class="search" id="inv-search" placeholder="Ara" value="${esc(inventoryFilter)}"></div>
      ${grid}
    </div>
    <div class="problems">Riot tam fiyat listesini artık yayınlamıyor. Mağazanda şu an görünen eşyaların (günlük mağaza, Night Market, paketler, mini paket mağazası) gerçek fiyatı kullanılır; diğer skinler kademe fiyatıyla (Select 875, Deluxe 1275, Premium 1775, Exclusive 2175, Ultra 2475; bıçaklar iki katı) tahmin edilir ve ~ ile işaretlenir. Battle Pass, kontrat ve ödül olarak gelenlerin fiyatı yoktur.${inv.note ? `<br>${esc(inv.note)}` : ""}</div>`;
  if (focusId === "inv-search" && $("inv-search")) {
    const el = $("inv-search");
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
  }
}

async function loadInventory(force) {
  if (!inventoryData || force) {
    inventoryData = force ? inventoryData : null;
    renderInventory();
  }
  try {
    inventoryData = await call("inventory", { force });
  } catch (err) {
    $("inventory").innerHTML = `<div class="page-head"><button class="btn" id="inv-back">← Hesabım</button><h1>Hesap İçeriği ve Değeri</h1></div><div class="note">${esc(err)}</div>`;
    return;
  }
  renderInventory();
}

$("inventory").addEventListener("click", (e) => {
  if (e.target.closest("#inv-back")) return void showInventory(false);
  if (e.target.closest("#inv-refresh")) return void loadInventory(true);
  const chip = e.target.closest("[data-group]");
  if (chip) {
    inventoryGroup = chip.dataset.group;
    renderInventory();
  }
});

$("inventory").addEventListener("input", (e) => {
  if (e.target.id !== "inv-search") return;
  inventoryFilter = e.target.value;
  renderInventory();
});

// ---------- updates ----------

async function checkUpdate() {
  try {
    const v = await call("app_version");
    $("version").textContent = v ? `v${v}` : "";
  } catch { /* ignore */ }
  let info = null;
  try {
    info = await call("check_update");
  } catch {
    return;
  }
  if (!info) return;
  const el = toast("update", `<b>Yeni sürüm hazır: v${esc(info.version)}</b>
    <span>Tek tıkla indirip kurar, uygulama kendini yeniden başlatır.</span>
    <button class="btn primary" id="do-update">Güncelle</button>`, true);
  el.querySelector("#do-update").addEventListener("click", async (e) => {
    e.target.disabled = true;
    e.target.textContent = "İndiriliyor…";
    try {
      await call("apply_update", { info });
    } catch (err) {
      toast("update", `<b>Güncelleme yapılamadı</b><span>${esc(err)}</span>`, true);
    }
  });
}

// ---------- start ----------

paintSwitch();
call("set_advanced", { on: advanced }).catch(() => {}).finally(poll);
checkUpdate();
