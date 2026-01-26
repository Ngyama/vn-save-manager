export interface Game {
    id: string;
    name: string;
    exe_path?: string;
    game_folder_path: string;
    save_folder_path?: string;
    cover_image?: string;
    save_mode?: string;  // 存档模式：single_file, folder, file_group, container
    save_config?: string;  // JSON 配置字符串
}

export interface Snapshot {
    id: string;
    game_id: string;
    name: string;
    original_save_path: string;
    backup_save_path: string;
    note?: string;
    created_at: string;
}

export interface Screenshot {
  id: string;
  game_id: string;
  name: string;
  image_path: string;
  note?: string;
  created_at: string;
}


