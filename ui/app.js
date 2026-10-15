"use strict";

const POLL_MS = 3000;
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

let selected = store.get("weapon") || FAVORITES[0];
let last = null;
let lastKey = "";

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

function img(src, cls, alt = "") {
  return src ? `<img class="${cls}" src="${esc(src)}" alt="${esc(alt)}" loading="lazy" draggable="false">` : "";
}

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
    `<button class="chip" data-id="${esc(id)}" aria-pressed="${id === selected}">${img(icon, "", "")}${esc(label)}</button>`;
  nav.innerHTML =
    chip(ALL, "Tümü", null) + weapons.map((w) => chip(w.id, w.name, w.icon)).join("");
}

function playerName(p) {
  if (p.hidden) {
    return `<div class="name hidden">Gizli oyuncu</div>`;
  }
  const tag = p.tag ? `<span class="tag">#${esc(p.tag)}</span>` : "";
  const you = p.is_me ? `<span class="you">SEN</span>` : "";
  return `<div class="name">${esc(p.name || "Oyuncu")}${tag}${you}</div>`;
}

function skinBlock(w) {
  if (!w) return `<div class="skin"><span class="none">Silah bilgisi yok</span></div>`;
  const tier = w.tier_color ? ` style="--tier:${esc(w.tier_color)}"` : "";
  return `<div class="skin${w.standard ? " standard" : ""}"${tier}>
    ${img(w.image, "", w.skin)}
    <div class="skin-name" title="${esc(w.skin)}">${esc(w.skin)}</div>
  </div>`;
}

function tile(w) {
  const tier = w.tier_color ? ` style="--tier:${esc(w.tier_color)}"` : "";
  return `<div class="tile${w.standard ? " standard" : ""}"${tier} title="${esc(w.skin)}">
    ${img(w.image, "", w.skin)}
    <div class="t-name">${esc(w.skin)}</div>
  </div>`;
}

function playerCard(p) {
  const meta = [p.agent, p.level != null ? `Seviye ${p.level}` : null]
    .filter(Boolean).map((x) => `<span>${esc(x)}</span>`).join("");
  const right = selected === ALL ? "<div></div>" : skinBlock(p.weapons.find((w) => w.weapon_id === selected));
  const grid = selected === ALL
    ? (p.weapons.length
        ? `<div class="grid">${p.weapons.map(tile).join("")}</div>`
        : `<div class="grid"><div class="t-name">Silah bilgisi yok</div></div>`)
    : "";
  return `<article class="player${p.is_me ? " me" : ""}">
    <div class="row">
      ${p.agent_icon ? img(p.agent_icon, "agent", p.agent || "") : `<div class="agent"></div>`}
      <div class="who">${playerName(p)}<div class="meta">${meta || "<span>Ajan seçiliyor</span>"}</div></div>
      ${right}
    </div>
    ${grid}
  </article>`;
}

function render(snap) {
  const status = $("status");
  status.dataset.state = snap.status;
  $("status-text").textContent = STATUS_TEXT[snap.status] || snap.status;

  const match = [snap.map && `<b>${esc(snap.map)}</b>`, snap.queue && esc(snap.queue)].filter(Boolean);
  $("match").innerHTML = match.join(" · ");

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
    : `<div class="note">${snap.status === "pregame" ? "Rakiplerin skinleri maç başlayınca görünür." : "Rakip yok."}</div>`;
}

function show(snap) {
  const key = JSON.stringify(snap);
  last = snap;
  if (key === lastKey) return;
  lastKey = key;
  render(snap);
}

$("weapons").addEventListener("click", (e) => {
  const btn = e.target.closest(".chip");
  if (!btn) return;
  selected = btn.dataset.id;
  store.set("weapon", selected);
  if (last) render(last);
});

const tauri = window.__TAURI__;

async function poll() {
  if (!document.hidden) {
    try {
      show(await tauri.core.invoke("snapshot"));
    } catch (err) {
      show({ status: "error", message: String(err), players: [], weapons: [] });
    }
  }
  setTimeout(poll, POLL_MS);
}

if (tauri) {
  poll();
} else {
  // Opened in a normal browser: show sample data so the layout can be checked.
  import("./demo.js").then((m) => show(m.default)).catch(() => {});
}
