import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface UserSettings {
  deviceName: string;
  saveDir: string;
  configured: boolean;
}

export interface Session {
  settings: UserSettings;
  authCode: string;
  receiverPort: number | null;
  receiverRunning: boolean;
  localIp: string | null;
  defaultPort: number;
}

export interface Peer {
  instance: string;
  deviceName: string;
  os: string;
  version: string;
  addresses: string[];
  port: number;
}

export type Direction = "send" | "recv";

export interface TransferStarted {
  id: string;
  direction: Direction;
  peer: string;
  fileCount: number;
  totalBytes: number;
}

export interface TransferProgress {
  id: string;
  direction: Direction;
  currentFile: string;
  currentBytesDone: number;
  currentBytesTotal: number;
  totalBytesDone: number;
  totalBytes: number;
  filesDone: number;
  filesTotal: number;
}

export interface TransferCompleted {
  id: string;
  direction: Direction;
  success: boolean;
  error: string | null;
  elapsedMs: number;
  totalBytes: number;
}

export interface IncomingRequest {
  id: string;
  peer: string;
  deviceName: string;
  os: string;
  fileCount: number;
  totalBytes: number;
}

export interface ReceiverReady {
  port: number;
  localIp: string | null;
  saveDir: string;
}

export const getSession = () => invoke<Session>("get_session");
export const saveSettings = (deviceName: string, saveDir: string) =>
  invoke<UserSettings>("save_settings", { deviceName, saveDir });
export const regenerateCode = () => invoke<string>("regenerate_code");
export const ensureReceiver = () => invoke<number>("ensure_receiver");
export const respondIncoming = (
  id: string,
  accept: boolean,
  overrideSaveDir?: string,
) => invoke<void>("respond_incoming", { id, accept, overrideSaveDir });
export const sendPaths = (
  peerIp: string,
  paths: string[],
  authCode: string,
  port?: number,
) => invoke<string>("send_paths", { peerIp, paths, authCode, port });

export const onTransferStarted = (cb: (e: TransferStarted) => void) =>
  listen<TransferStarted>("transfer://started", (e) => cb(e.payload));
export const onTransferProgress = (cb: (e: TransferProgress) => void) =>
  listen<TransferProgress>("transfer://progress", (e) => cb(e.payload));
export const onTransferCompleted = (cb: (e: TransferCompleted) => void) =>
  listen<TransferCompleted>("transfer://completed", (e) => cb(e.payload));
export const onReceiverReady = (cb: (e: ReceiverReady) => void) =>
  listen<ReceiverReady>("receiver://ready", (e) => cb(e.payload));
export const onPeerAdded = (cb: (p: Peer) => void) =>
  listen<Peer>("peer://added", (e) => cb(e.payload));
export const onPeerRemoved = (cb: (instance: string) => void) =>
  listen<string>("peer://removed", (e) => cb(e.payload));
export const onIncomingRequest = (cb: (r: IncomingRequest) => void) =>
  listen<IncomingRequest>("incoming://request", (e) => cb(e.payload));

export type Unlisten = UnlistenFn;
