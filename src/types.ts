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
    original_save_path: string;
    backup_save_path: string;
    image_path?: string;
    text_content?: string;
    note?: string;
    created_at: string;
}


