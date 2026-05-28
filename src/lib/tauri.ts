import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export async function ping(name: string): Promise<string> {
  return invoke<string>("ping", { name });
}

export async function getLocalIp(): Promise<string | null> {
  return invoke<string | null>("get_local_ip");
}

export async function getDefaultPort(): Promise<number> {
  return invoke<number>("get_default_port");
}

export async function getDeviceName(): Promise<string> {
  return invoke<string>("get_device_name");
}

export async function startReceiving(
  saveDir: string,
  port?: number,
): Promise<number> {
  return invoke<number>("start_receiving", { saveDir, port });
}

export async function stopReceiving(): Promise<void> {
  return invoke<void>("stop_receiving");
}

export async function sendFile(
  peerIp: string,
  filePath: string,
  port?: number,
): Promise<string> {
  return invoke<string>("send_file", { peerIp, filePath, port });
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

export interface ReceiverReady {
  port: number;
  localIp: string | null;
  saveDir: string;
}

export const onTransferStarted = (cb: (e: TransferStarted) => void) =>
  listen<TransferStarted>("transfer://started", (e) => cb(e.payload));

export const onTransferProgress = (cb: (e: TransferProgress) => void) =>
  listen<TransferProgress>("transfer://progress", (e) => cb(e.payload));

export const onTransferCompleted = (cb: (e: TransferCompleted) => void) =>
  listen<TransferCompleted>("transfer://completed", (e) => cb(e.payload));

export const onReceiverReady = (cb: (e: ReceiverReady) => void) =>
  listen<ReceiverReady>("receiver://ready", (e) => cb(e.payload));

export type Unlisten = UnlistenFn;
