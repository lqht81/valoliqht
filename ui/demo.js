// Sample data used only when the page is opened outside the app.
const svg = (body, vb = "0 0 200 50") =>
  "data:image/svg+xml," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}">${body}</svg>`);
const gun = (c) => svg(`<path fill="${c}" d="M8 20h120l8-6h40v8h16v6h-30l-6 4h-22l-4 14h-16l2-14H60l-8 10H30l6-10H8z"/>`);
const face = (c) => svg(`<rect width="44" height="44" fill="${c}"/><circle cx="22" cy="17" r="8" fill="#0d1117" opacity=".5"/><rect x="9" y="29" width="26" height="15" rx="7" fill="#0d1117" opacity=".5"/>`, "0 0 44 44");
const gem = (c) => svg(`<path fill="${c}" d="M20 2 36 16 20 38 4 16z"/><path fill="#fff" opacity=".35" d="M20 2 28 16 20 38z"/>`, "0 0 40 40");
const buddy = (c) => svg(`<circle cx="20" cy="20" r="12" fill="${c}"/><circle cx="20" cy="6" r="3" fill="#ccc"/>`, "0 0 40 40");
const art = (a, b) => svg(`<defs><linearGradient id="g" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="400" height="100" fill="url(#g)"/>`, "0 0 400 100");

const tier = (t, name, color) => ({ tier: t, name, color, icon: gem(color) });
const GOLD2 = tier(13, "ALTIN 2", "#e8b84a");
const PLAT1 = tier(15, "PLATİN 1", "#4fb3c2");
const DIA3 = tier(20, "ELMAS 3", "#c58cff");
const ASC1 = tier(21, "YÜCELİŞ 1", "#4fd77f");
const IMM2 = tier(25, "ÖLÜMSÜZLÜK 2", "#e05a7a");
const SIL3 = tier(11, "GÜMÜŞ 3", "#c8ccd0");

const rank = (current, rr, peak) => ({ current, rr, peak, wins: 9, games: 15, leaderboard: null });

const V = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const P = "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a";
const O = "a03b24d3-4319-996d-0f8c-94bbfba1dfc7";
const S = "e336c6b8-418d-9340-d77f-7a9e4cfe0702";
const K = "2f59173c-4bed-b6c3-2191-dea9b58be9c7";

const w = (id, [name, color, b]) => ({
  weapon_id: id, skin: name, image: gun(color || "#7a8494"), tier_color: color || null,
  standard: !color, buddy: b ? buddy(b) : null,
});
const loadout = (v, p, o, s, k) => [w(V, v), w(P, p), w(O, o), w(S, s), w(K, k)];
const std = (n) => [`Standart ${n}`, null];

const player = (name, tag, agent, color, ally, extra, weapons) => ({
  puuid: name || agent, name, tag, hidden: false, agent, agent_icon: face(color), ally,
  is_me: false, level: 120, card: face("#2b3a55"), title: null, rank: null, party: null,
  weapons, ...extra,
});

const snapshot = {
  status: "ingame",
  message: null,
  match_id: "demo",
  map: "Ascent",
  queue: "Rekabetçi",
  score: [7, 5],
  weapons: [
    { id: V, name: "Vandal", category: "Rifle", icon: gun("#fff") },
    { id: P, name: "Phantom", category: "Rifle", icon: gun("#fff") },
    { id: O, name: "Operator", category: "Sniper", icon: gun("#fff") },
    { id: S, name: "Sheriff", category: "Sidearm", icon: gun("#fff") },
    { id: K, name: "Yakın Dövüş", category: "Melee", icon: gun("#fff") },
  ],
  players: [
    player("Liqht", "TR1", "Jett", "#8ad7ff", true, { is_me: true, level: 212, title: "Sıfırdan Kahraman", rank: rank(DIA3, 47, ASC1), party: 1 },
      loadout(["Prime Vandal Mavi", "#d1548d", "#f2c14e"], ["Ion Phantom", "#f5955b"], std("Operator"), ["Reaver Sheriff", "#f5955b"], ["Elderflame Hançer", "#f5955b"])),
    player("Kanka", "0001", "Sage", "#9ff2d1", true, { rank: rank(PLAT1, 12, DIA3), party: 1 },
      loadout(["Reaver Vandal", "#f5955b", "#6aa4ff"], std("Phantom"), ["Glitchpop Operator", "#d1548d"], std("Sheriff"), std("Bıçak"))),
    player("Mert", "7777", "Omen", "#7d6bd1", true, { level: null, rank: rank(GOLD2, 88, null) },
      loadout(std("Vandal"), ["Oni Phantom", "#d1548d"], std("Operator"), ["Ion Sheriff", "#d1548d"], ["Karambit", "#fad663"])),
    player(null, null, "Reyna", "#c26bd1", false, { hidden: true, level: null, rank: rank(IMM2, 30, IMM2), party: 2, title: "Yapay Zekâ" },
      loadout(["Champions 2023 Vandal", "#fad663", "#ff6b6b"], std("Phantom"), std("Operator"), std("Sheriff"), std("Bıçak"))),
    player("ProEnemy", "EU", "Raze", "#f2a35b", false, { rank: rank(ASC1, 61, IMM2), party: 2 },
      loadout(["Kuronami Vandal", "#fad663"], ["RGX 11z Pro Phantom", "#f5955b"], std("Operator"), std("Sheriff"), ["RGX Kesik", "#f5955b"])),
    player("Yalnız", "777", "Sova", "#5fa8ff", false, {},
      loadout(std("Vandal"), std("Phantom"), std("Operator"), std("Sheriff"), std("Bıçak"))),
  ],
};

const game = (agent, color, map, won, us, them, k, d, a, acs) => ({
  match_id: map + k, map, queue: "Rekabetçi", agent, agent_icon: face(color), won,
  kills: k, deaths: d, assists: a, acs, hs: 24, rounds_us: us, rounds_them: them, started: 0, tier: null,
});

const stats = {
  kd: 1.32, acs: 241, hs: 24, winrate: 62,
  agents: [
    { name: "Jett", icon: face("#8ad7ff"), count: 4 },
    { name: "Raze", icon: face("#f2a35b"), count: 3 },
    { name: "Omen", icon: face("#7d6bd1"), count: 1 },
  ],
  matches: [
    game("Jett", "#8ad7ff", "Ascent", true, 13, 9, 24, 15, 4, 287),
    game("Raze", "#f2a35b", "Bind", false, 10, 13, 17, 18, 6, 221),
    game("Jett", "#8ad7ff", "Lotus", true, 13, 11, 21, 16, 3, 254),
    game("Omen", "#7d6bd1", "Haven", true, 13, 4, 15, 8, 11, 219),
  ],
};

const friend = (name, tag, status, extra = {}) => ({
  puuid: name, name, tag, note: null, online: status !== "Çevrimdışı", in_valorant: true,
  status, map: null, queue: null, party_size: null, score: null, rank: null, level: null, ...extra,
});

const friends = [
  friend("Kanka", "0001", "Maçta", { map: "Ascent", queue: "Rekabetçi", score: [7, 5], party_size: 2, rank: PLAT1, level: 160 }),
  friend("Mert", "7777", "Ajan seçimi", { queue: "Derecesiz", rank: GOLD2, level: 98 }),
  friend("Zeynep", "TR", "Menüde", { queue: "Rekabetçi", rank: SIL3, level: 43, note: "okul arkadaşı" }),
  friend("Lolcu", "EUW", "League of Legends'ta", { in_valorant: false }),
  friend("Uyuyan", "000", "Çevrimdışı", { in_valorant: false }),
];

const offer = (name, color, price, extra = {}) => ({ name, image: gun(color), tier_color: color, price, original: null, discount: null, ...extra });

const account = {
  puuid: "Liqht",
  name: "Liqht",
  tag: "TR1",
  level: 212,
  card_wide: art("#1d3b5c", "#5a2a6b"),
  card_small: face("#2b3a55"),
  title: "Sıfırdan Kahraman",
  rank: { current: DIA3, rr: 47, peak: ASC1, wins: 18, games: 31, leaderboard: null },
  rr_history: [
    { map: "Ascent", tier: DIA3, rr_after: 47, earned: 21, started: 0 },
    { map: "Bind", tier: DIA3, rr_after: 26, earned: -17, started: 0 },
    { map: "Lotus", tier: DIA3, rr_after: 43, earned: 19, started: 0 },
    { map: "Haven", tier: DIA3, rr_after: 24, earned: 24, started: 0 },
  ],
  wallet: { vp: 1250, rp: 40, kc: 3210 },
  store: {
    daily: [
      offer("Prime Vandal", "#d1548d", 1775),
      offer("Reaver Sheriff", "#f5955b", 1775),
      offer("Oni Phantom", "#d1548d", 1775),
      offer("Glitchpop Bıçak", "#fad663", 4350),
    ],
    daily_ends_in: 11 * 3600 + 25 * 60,
    night: [
      offer("Ion Operator", "#f5955b", 1242, { original: 1775, discount: 30 }),
      offer("Sakura Classic", "#5fe0e8", 765, { original: 875, discount: 13 }),
    ],
    night_ends_in: 4 * 86400 + 5 * 3600,
    bundles: [{ name: "Kuronami", image: art("#1b2b4a", "#3ee0b0"), ends_in: 2 * 86400 }],
  },
  collection: [
    { name: "Elderflame Hançer", weapon: "Yakın Dövüş", image: gun("#fad663"), tier_color: "#fad663" },
    { name: "Prime Vandal", weapon: "Vandal", image: gun("#d1548d"), tier_color: "#d1548d" },
    { name: "Ion Phantom", weapon: "Phantom", image: gun("#f5955b"), tier_color: "#f5955b" },
    { name: "Reaver Sheriff", weapon: "Sheriff", image: gun("#f5955b"), tier_color: "#f5955b" },
    { name: "Sakura Classic", weapon: "Classic", image: gun("#5fe0e8"), tier_color: "#5fe0e8" },
  ],
  contracts: [
    { name: "Bölüm 9 Battle Pass", kind: "Battle Pass", level: 31, total: 55 },
    { name: "Waylay", kind: "Aktif kontrat", level: 4, total: 10 },
  ],
  problems: [],
};

export default { snapshot, stats, friends, account };
