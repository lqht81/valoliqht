"use strict";

const POLL_MS = 3000;
const FRIENDS_MS = 10000;
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
  menu: "Menüde",
  pregame: "Ajan seçimi",
  ingame: "Maçta",
  error: "Hata",
};

const EMPTY_TITLE = {
  riot_closed: "Riot Client açık değil",
  waiting: "VALORANT bekleniyor",
  menu: "Maç bekleniyor",
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
  await new Promise((r) => setTimeout(r, 150));
  if (cmd === "snapshot") return demo.snapshot;
  if (cmd === "player_stats") return demo.stats;
  if (cmd === "friends") return demo.friends;
  if (cmd === "account") return demo.account;
  if (cmd === "set_advanced") return null;
  throw new Error("unknown command");
}

// ---------- helpers ----------

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

function img(src, cls = "", alt = "") {
  return src ? `<img class="${cls}" src="${esc(src)}" alt="${esc(alt)}" loading="lazy" draggable="false">` : "";
}

function tierStyle(color) {
  return color ? ` style="--tier:${esc(color)}"` : "";
}

function tierName(name) {
  // valorant-api gives upper-case names; show them in title case.
  return String(name || "").toLocaleLowerCase("tr-TR").replace(/(^|\s)(\S)/g, (m, a, b) => a + b.toLocaleUpperCase("tr-TR"));
}

function duration(sec) {
  if (sec == null) return "";
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d > 0) return `${d} gün ${h} sa`;
  if (h > 0) return `${h} sa ${m} dk`;
  return `${m} dk`;
}

function num(n) {
  return n == null ? "—" : Number(n).toLocaleString("tr-TR");
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

function askRisk() {
  $("risk").hidden = false;
  $("risk-cancel").focus();
}

$("advanced").addEventListener("click", () => {
  if (advanced) setAdvanced(false);
  else askRisk();
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
  if (next === "account") loadAccount(false);
}

$("tabs").addEventListener("click", (e) => {
  const b = e.target.closest(".tab");
  if (b) setTab(b.dataset.tab);
});

// ---------- match ----------

let selected = store.get("weapon") || FAVORITES[0];
let last = null;
let lastKey = "";
const open = new Set();
const statsCache = new Map(); // puuid -> {state:"loading"|"done"|"error", data|error}

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

function statsBlock(puuid) {
  const st = statsCache.get(puuid);
  if (!st || st.state === "loading") return `<div class="stats"><div class="loading">Son maçlar yükleniyor…</div></div>`;
  if (st.state === "error") return `<div class="stats"><div class="err">${esc(st.error)}</div></div>`;
  const s = st.data;
  if (!s.matches.length) return `<div class="stats"><div class="err">Yakın zamanda oynanmış maç bulunamadı.</div></div>`;
  const chip = (label, value) => `<span class="stat">${label}<b>${value}</b></span>`;
  const mains = s.agents.map((a) => img(a.icon, "", `${a.name} (${a.count})`).replace("<img", `<img title="${esc(a.name)} · ${a.count} maç"`)).join("");
  const games = s.matches.map((m) => {
    const cls = m.won == null ? "" : m.won ? " win" : " loss";
    const res = m.won == null ? "—" : `${m.rounds_us}-${m.rounds_them}`;
    return `<div class="game${cls}">${img(m.agent_icon, "", m.agent || "")}
      <span class="map">${esc(m.map || "?")} · ${esc(m.queue)}</span>
      <span class="res">${res}</span>
      <span class="kda">${m.kills}/${m.deaths}/${m.assists}</span>
      <span class="acs">${m.acs} ACS</span></div>`;
  }).join("");
  return `<div class="stats">
    <div class="stat-row">
      ${chip("K/D", s.kd ?? "—")}${chip("ACS", s.acs ?? "—")}${chip("HS", s.hs != null ? s.hs + "%" : "—")}${chip("Kazanma", s.winrate != null ? s.winrate + "%" : "—")}
      <span class="mains">${mains}</span>
    </div>
    <div class="games">${games}</div>
  </div>`;
}

function playerCard(p) {
  const sub = [p.title && `<span class="title">${esc(p.title)}</span>`, p.agent && esc(p.agent), p.level != null && `Seviye ${p.level}`]
    .filter(Boolean).join(" · ");
  const right = selected === ALL ? "<div></div>" : skinBlock(p.weapons.find((w) => w.weapon_id === selected));
  const grid = selected === ALL
    ? `<div class="grid">${p.weapons.length ? p.weapons.map(tile).join("") : `<div class="t-name">Silah bilgisi yok</div>`}</div>`
    : "";
  const face = p.agent_icon ? img(p.agent_icon, "agent", p.agent || "") : `<div class="agent"></div>`;
  const clickable = advanced || p.is_me;
  return `<article class="player${p.is_me ? " me" : ""}${clickable ? "" : " static"}" data-puuid="${esc(p.puuid)}"${advanced && p.party ? ` data-party="${((p.party - 1) % 5) + 1}"` : ""}>
    <div class="row">
      <div class="face">${face}${img(p.card, "card")}</div>
      <div class="who">${playerName(p)}<div class="sub">${sub || "Ajan seçiliyor"}</div>${rankLine(p.rank)}</div>
      ${right}
    </div>
    ${grid}
    ${open.has(p.puuid) ? statsBlock(p.puuid) : ""}
  </article>`;
}

function renderMatch(snap) {
  $("status").dataset.state = snap.status;
  $("status-text").textContent = STATUS_TEXT[snap.status] || snap.status;

  const parts = [];
  if (snap.map) parts.push(`<b>${esc(snap.map)}</b>`);
  if (snap.queue) parts.push(esc(snap.queue));
  let html = parts.join(" · ");
  if (snap.score) html += ` <span class="score"><span class="us">${snap.score[0]}</span> – <span class="them">${snap.score[1]}</span></span>`;
  $("match").innerHTML = html;

  const inMatch = (snap.status === "pregame" || snap.status === "ingame") && snap.players.length;
  $("empty").hidden = !!inMatch;
  $("teams").hidden = !inMatch;
  renderWeapons(snap);

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
let myStats = null;
let collectionFilter = "";

function offerCard(o) {
  const price = o.discount
    ? `<b>${num(o.price)} VP</b><s>${num(o.original)}</s><span class="off">-%${o.discount}</span>`
    : `<b>${num(o.price)} VP</b>`;
  return `<div class="offer"${tierStyle(o.tier_color)}>
    ${img(o.image, "", o.name)}
    <div class="o-name" title="${esc(o.name)}">${esc(o.name)}</div>
    <div class="price">${price}</div>
  </div>`;
}

function ownedGrid(a) {
  const q = collectionFilter.toLocaleLowerCase("tr-TR");
  const items = a.collection.filter((s) => !q || `${s.name} ${s.weapon}`.toLocaleLowerCase("tr-TR").includes(q));
  if (!items.length) return `<div class="note">${a.collection.length ? "Eşleşen skin yok." : "Koleksiyon boş."}</div>`;
  return `<div class="owned">${items.map((s) => `<div class="own"${tierStyle(s.tier_color)} title="${esc(s.name)}">
    ${img(s.image, "", s.name)}<div class="o-name">${esc(s.name)}</div><div class="o-weapon">${esc(s.weapon)}</div></div>`).join("")}</div>`;
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
    recent = statsBlock(a.puuid);
  } else if (myStats && myStats.state === "error") {
    recent = `<div class="note">${esc(myStats.error)}</div>`;
  }

  box.innerHTML = `
    <div class="hero">
      ${img(a.card_wide, "banner")}
      <div class="hero-in">
        ${a.card_small ? img(a.card_small, "avatar") : `<div class="avatar"></div>`}
        <div>
          <h1>${esc(a.name || "Hesabım")}${a.tag ? `<span class="tag">#${esc(a.tag)}</span>` : ""}</h1>
          <div class="meta">${a.title ? `<span class="title">${esc(a.title)}</span>` : ""}${a.level != null ? `<span>Seviye ${a.level}</span>` : ""}</div>
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

    ${st ? `<div class="section"><h2>Günlük mağaza<span class="timer">${duration(st.daily_ends_in)} sonra yenilenir</span></h2>
      <div class="offers">${st.daily.map(offerCard).join("")}</div></div>` : ""}

    ${st && st.night.length ? `<div class="section"><h2>Night Market<span class="timer">${duration(st.night_ends_in)} kaldı</span></h2>
      <div class="offers">${st.night.map(offerCard).join("")}</div></div>` : ""}

    ${st && st.bundles.length ? `<div class="section"><h2>Öne çıkan paket</h2>${st.bundles.map((b) => `<div class="bundle">${img(b.image)}<div>
      <div class="o-name" style="font:600 18px var(--head)">${esc(b.name)}</div>
      <div class="muted">${b.ends_in != null ? duration(b.ends_in) + " kaldı" : ""}</div></div></div>`).join("")}</div>` : ""}

    <div class="two">
      <div class="section"><h2>RR geçmişi</h2>${rr}</div>
      <div class="section"><h2>Kontratlar</h2>${contracts}</div>
    </div>

    <div class="section"><h2>Son maçların</h2>${recent}</div>

    <div class="section">
      <div class="collection-head"><h2>Koleksiyon</h2>
        <input class="search" id="skin-search" placeholder="Skin ya da silah ara" value="${esc(collectionFilter)}">
      </div>
      <div id="owned">${ownedGrid(a)}</div>
    </div>

    ${a.problems.length ? `<div class="problems">Yüklenemeyenler: ${a.problems.map(esc).join(" · ")}</div>` : ""}
  `;

  box.insertAdjacentHTML("afterbegin", `<div class="page-head"><h1>Hesabım</h1><button class="btn" id="account-refresh">Yenile</button></div>`);
}

async function loadAccount(force) {
  const box = $("account");
  if (!accountData) box.innerHTML = `<div class="note">Hesap bilgileri yükleniyor…</div>`;
  try {
    accountData = await call("account", { force });
    renderAccount();
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
  if (e.target.closest("#account-refresh")) loadAccount(true);
});

$("account").addEventListener("input", (e) => {
  if (e.target.id !== "skin-search" || !accountData) return;
  collectionFilter = e.target.value;
  $("owned").innerHTML = ownedGrid(accountData);
});

// ---------- start ----------

paintSwitch();
call("set_advanced", { on: advanced }).catch(() => {}).finally(poll);
