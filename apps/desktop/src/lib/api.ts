import { invoke } from "@tauri-apps/api/core";

export type ImageValidation = {
  width: number;
  height: number;
  bytes: number;
  format: string;
  square: boolean;
  power_of_two: boolean;
  allowed_size: boolean;
  under_size_limit: boolean;
  compatible: boolean;
  messages: string[];
};

export type ProcessedImage = {
  preview_data_url: string;
  validation: ImageValidation;
  png_base64: string;
};

export type SteamStatus = {
  mode: "dry_run" | "live";
  steam_running: boolean;
  initialized: boolean;
  app_id: number;
  rust_owned: boolean;
  steam_id: string | null;
  persona_name: string | null;
  messages: string[];
  ready_to_publish: boolean;
};

export type PrepareResponse = {
  project_id: string;
  asset_dir: string;
  validation: ImageValidation;
  preview_data_url: string;
};

export type PublishResult = {
  published_file_id: number;
  url: string;
  dry_run: boolean;
  needs_workshop_agreement: boolean;
  messages: string[];
};

export type ProjectRecord = {
  id: string;
  title: string;
  description: string;
  item_type: string;
  asset_dir: string;
  published_file_id: string | null;
  created_at: string;
  updated_at: string;
};

export type HistoryEntry = {
  id: string;
  project_id: string;
  published_file_id: string;
  url: string;
  dry_run: boolean;
  success: boolean;
  message: string;
  created_at: string;
};

export async function processImagePath(
  path: string,
  options: { target_size?: number; rotate?: number } = {},
): Promise<ProcessedImage> {
  return invoke("process_image_path", { path, options });
}

export async function processImageBytes(
  bytesBase64: string,
  options: { target_size?: number; rotate?: number } = {},
): Promise<ProcessedImage> {
  return invoke("process_image_bytes", {
    bytesBase64,
    options,
  });
}

export async function prepareAsset(request: {
  png_base64: string;
  title: string;
  description: string;
  item_type?: string;
  project_id?: string;
}): Promise<PrepareResponse> {
  return invoke("prepare_asset", { request });
}

export async function steamDoctor(dryRun: boolean): Promise<SteamStatus> {
  return invoke("steam_doctor", { dryRun });
}

export async function publishAsset(request: {
  asset_dir: string;
  project_id: string;
  title: string;
  description: string;
  dry_run: boolean;
  /** true = CreateItem (unique SkinID). false = update existing id */
  create_new?: boolean;
  published_file_id?: number;
  visibility?: string;
}): Promise<PublishResult> {
  return invoke("publish_asset", { request });
}

export async function listProjects(): Promise<ProjectRecord[]> {
  return invoke("list_projects");
}

export async function listHistory(projectId?: string): Promise<HistoryEntry[]> {
  return invoke("list_history", { projectId: projectId ?? null });
}

export async function appPaths(): Promise<{ data_dir: string; projects_dir: string }> {
  return invoke("app_paths");
}

export async function wipeLocalData(): Promise<void> {
  return invoke("wipe_local_data");
}

export function bytesToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}
