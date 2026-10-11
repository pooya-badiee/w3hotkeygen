// The one place the GitHub repo is named. Downloads point at the latest
// release, which the release workflow fills with w3hk.exe and its SHA-256.
export const REPO = "https://github.com/pooya-badiee/w3hotkeygen";
export const EXE = `${REPO}/releases/latest/download/w3hk.exe`;
export const SHA = `${REPO}/releases/latest/download/w3hk.exe.sha256`;

// One button on a 4x3 command card.
export interface Cell {
  name: string;
  key: string;
  // Drawn with a gold (good) or red (bad) border.
  mark?: "good" | "bad";
}

export interface Card {
  title: string;
  // 3 rows of 4; null is an empty cell.
  rows: (Cell | null)[][];
}

// The Blood Mage's main card, as each layout sets it up. The cells are
// the game's own (Classic, Blizzard's Grid) and the generator's output.
export const cards: Card[] = [
  {
    title: "Classic",
    rows: [
      [
        { name: "Move", key: "M" },
        { name: "Stop", key: "S" },
        { name: "Hold", key: "H" },
        { name: "Attack", key: "A", mark: "good" },
      ],
      [{ name: "Patrol", key: "P" }, null, null, { name: "Learn", key: "O" }],
      [
        { name: "Flame Strike", key: "F", mark: "bad" },
        { name: "Banish", key: "B", mark: "bad" },
        { name: "Siphon Mana", key: "N", mark: "bad" },
        { name: "Phoenix", key: "X", mark: "bad" },
      ],
    ],
  },
  {
    title: "Blizzard's Grid",
    rows: [
      [
        { name: "Move", key: "Q" },
        { name: "Stop", key: "W" },
        { name: "Hold", key: "E" },
        { name: "Attack", key: "R", mark: "bad" },
      ],
      [{ name: "Patrol", key: "A", mark: "bad" }, null, null, { name: "Learn", key: "F" }],
      [
        { name: "Flame Strike", key: "Z" },
        { name: "Banish", key: "X" },
        { name: "Siphon Mana", key: "C" },
        { name: "Phoenix", key: "V" },
      ],
    ],
  },
  {
    title: "WC3 Grid Hotkeys",
    rows: [
      [
        { name: "Flame Strike", key: "Q", mark: "good" },
        { name: "Banish", key: "W", mark: "good" },
        { name: "Siphon Mana", key: "E", mark: "good" },
        { name: "Phoenix", key: "R", mark: "good" },
      ],
      [
        { name: "Attack", key: "A", mark: "good" },
        { name: "Stop", key: "S" },
        { name: "Move", key: "M" },
        { name: "Hold", key: "H" },
      ],
      [{ name: "Patrol", key: "P" }, null, null, { name: "Learn", key: "V" }],
    ],
  },
];
