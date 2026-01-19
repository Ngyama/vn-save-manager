import { Game } from "../types";

interface EditGameModalProps {
  show: boolean;
  game: Game | null;
  savePath: string;
  exePath: string;
  onClose: () => void;
  onSavePathChange: (path: string) => void;
  onExePathChange: (path: string) => void;
  onBrowseSaveFolder: () => void;
  onBrowseExeFile: () => void;
  onSubmit: (gameId: string, name: string, saveFolderPath: string | undefined, exeFilePath: string | undefined) => void;
}

export default function EditGameModal({
  show,
  game,
  savePath,
  exePath,
  onClose,
  onSavePathChange,
  onExePathChange,
  onBrowseSaveFolder,
  onBrowseExeFile,
  onSubmit,
}: EditGameModalProps) {
  if (!show || !game) return null;

  return (
    <div className="fixed inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        <div className="p-6 border-b border-gray-200">
          <h3 className="text-xl font-semibold text-gray-900">编辑游戏</h3>
        </div>
        
        <form
          onSubmit={(e) => {
            e.preventDefault();
            const form = e.target as HTMLFormElement;
            const gameName = form.gameName.value;
            const newSavePath = form.savePath.value.trim() || undefined;
            const newExePath = form.exePath.value.trim() || undefined;
            onSubmit(game.id, gameName, newSavePath, newExePath);
          }}
          className="p-6 space-y-5"
        >
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              游戏名称
            </label>
            <input
              name="gameName"
              placeholder="输入游戏名称"
              defaultValue={game.name}
              required
              className="w-full px-4 py-3 bg-gray-50 border border-gray-200 rounded-xl text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              存档文件夹（留空则不修改）
            </label>
            <div className="flex gap-2">
              <input
                name="savePath"
                placeholder={game.save_folder_path || "请选择存档文件夹"}
                value={savePath}
                onChange={(e) => onSavePathChange(e.target.value)}
                className="flex-1 px-4 py-3 bg-gray-50 border border-gray-200 rounded-xl text-gray-600 focus:outline-none"
              />
              <button
                type="button"
                onClick={onBrowseSaveFolder}
                className="px-4 py-3 bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium rounded-xl transition-colors whitespace-nowrap"
              >
                浏览...
              </button>
            </div>
            {game.save_folder_path && (
              <p className="text-xs text-gray-500 mt-1">当前: {game.save_folder_path}</p>
            )}
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              游戏可执行文件 (.exe)（留空则不修改）
            </label>
            <div className="flex gap-2">
              <input
                name="exePath"
                placeholder={game.exe_path || "请选择游戏exe文件"}
                value={exePath}
                onChange={(e) => onExePathChange(e.target.value)}
                className="flex-1 px-4 py-3 bg-gray-50 border border-gray-200 rounded-xl text-gray-600 focus:outline-none"
              />
              <button
                type="button"
                onClick={onBrowseExeFile}
                className="px-4 py-3 bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium rounded-xl transition-colors whitespace-nowrap"
              >
                浏览...
              </button>
            </div>
            {game.exe_path && (
              <p className="text-xs text-gray-500 mt-1">当前: {game.exe_path}</p>
            )}
          </div>
          
          <p className="text-xs text-gray-500 leading-relaxed">
            提示：如果路径字段留空，则不会修改该路径。修改路径后需要确保新路径存在且有效。
          </p>
          
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-6 py-2.5 bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium rounded-xl transition-colors"
            >
              取消
            </button>
            <button
              type="submit"
              className="px-6 py-2.5 bg-blue-500 hover:bg-blue-600 text-white font-medium rounded-xl shadow-sm hover:shadow-md transition-all"
            >
              保存
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
