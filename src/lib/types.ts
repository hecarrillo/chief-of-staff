export interface Message {
  id: string;
  from: "hector" | "cos" | "telegram" | "system";
  text: string;
  timestamp: string;
  forward_to_telegram: boolean;
  delivered?: boolean;
  image?: string;
  reply_to?: string;
  reply_preview?: string;
}

export interface SessionInfo {
  name: string;
  created: string;
  windows: number;
}

export interface WindowInfo {
  index: number;
  name: string;
  active: boolean;
}

export interface VaultFile {
  path: string;
  name: string;
  frontmatter: Record<string, string>;
  body: string;
}

export interface VaultChange {
  path: string;
  kind: string;
}

export type Mode = "at_desk" | "away";
