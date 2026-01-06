export interface Game {
    id: string;
    name: string;
    exe_path?: string;
    game_folder_path: string;
    save_folder_path?: string;
    cover_image?: string;
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


