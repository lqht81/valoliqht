// Sample data used only when the page is opened outside the app.
const gun = (c) =>
  "data:image/svg+xml," +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 50"><path fill="${c}" d="M8 20h120l8-6h40v8h16v6h-30l-6 4h-22l-4 14h-16l2-14H60l-8 10H30l6-10H8z"/></svg>`
  );
const face = (c) =>
  "data:image/svg+xml," +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 44 44"><rect width="44" height="44" fill="${c}"/><circle cx="22" cy="17" r="8" fill="#0d1117" opacity=".5"/><rect x="9" y="29" width="26" height="15" rx="7" fill="#0d1117" opacity=".5"/></svg>`
  );

const V = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const P = "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a";
const O = "a03b24d3-4319-996d-0f8c-94bbfba1dfc7";
const S = "e336c6b8-418d-9340-d77f-7a9e4cfe0702";
const K = "2f59173c-4bed-b6c3-2191-dea9b58be9c7";

const loadout = (v, p, o, s, k) => [
  { weapon_id: V, skin: v[0], image: gun(v[1] || "#7a8494"), tier_color: v[1], standard: !v[1] },
  { weapon_id: P, skin: p[0], image: gun(p[1] || "#7a8494"), tier_color: p[1], standard: !p[1] },
  { weapon_id: O, skin: o[0], image: gun(o[1] || "#7a8494"), tier_color: o[1], standard: !o[1] },
  { weapon_id: S, skin: s[0], image: gun(s[1] || "#7a8494"), tier_color: s[1], standard: !s[1] },
  { weapon_id: K, skin: k[0], image: gun(k[1] || "#7a8494"), tier_color: k[1], standard: !k[1] },
];

const std = (n) => [`Standart ${n}`, null];
const player = (name, tag, agent, color, ally, extra = {}, w) => ({
  puuid: name, name, tag, hidden: false, agent, agent_icon: face(color), ally,
  is_me: false, level: 120, weapons: w, ...extra,
});

export default {
  status: "ingame",
  message: null,
  map: "Ascent",
  queue: "Rekabetçi",
  weapons: [
    { id: V, name: "Vandal", category: "Rifle", icon: gun("#fff") },
    { id: P, name: "Phantom", category: "Rifle", icon: gun("#fff") },
    { id: O, name: "Operator", category: "Sniper", icon: gun("#fff") },
    { id: S, name: "Sheriff", category: "Sidearm", icon: gun("#fff") },
    { id: K, name: "Yakın Dövüş", category: "Melee", icon: gun("#fff") },
  ],
  players: [
    player("Liqht", "TR1", "Jett", "#8ad7ff", true, { is_me: true, level: 212 },
      loadout(["Prime Vandal Mavi", "#d1548d"], ["Ion Phantom", "#f5955b"], std("Operator"), ["Reaver Sheriff", "#f5955b"], ["Elderflame Hançer", "#f5955b"])),
    player("Kanka", "0001", "Sage", "#9ff2d1", true, {},
      loadout(["Reaver Vandal", "#f5955b"], std("Phantom"), ["Glitchpop Operator", "#d1548d"], std("Sheriff"), std("Bıçak"))),
    player("Mert", "7777", "Omen", "#7d6bd1", true, { level: null },
      loadout(std("Vandal"), ["Oni Phantom", "#d1548d"], std("Operator"), ["Ion Sheriff", "#d1548d"], ["Karambit", "#fad663"])),
    player(null, null, "Reyna", "#c26bd1", false, { hidden: true, level: null },
      loadout(["Champions 2023 Vandal", "#fad663"], std("Phantom"), std("Operator"), std("Sheriff"), std("Bıçak"))),
    player("ProEnemy", "EU", "Raze", "#f2a35b", false, {},
      loadout(["Kuronami Vandal", "#fad663"], ["RGX 11z Pro Phantom", "#f5955b"], std("Operator"), std("Sheriff"), ["RGX Kesik", "#f5955b"])),
  ],
};
