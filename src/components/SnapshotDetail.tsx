import { Snapshot } from "../types";

interface SnapshotDetailProps {
  snapshot: Snapshot;
  imageCache?: Record<string, string>;
  noteText: string;
  noteEditMode: boolean;
  onNoteTextChange: (text: string) => void;
  onNoteEditModeToggle: () => void;
}

export default function SnapshotDetail({
  snapshot,
  noteText,
  noteEditMode,
  onNoteTextChange,
  onNoteEditModeToggle,
}: SnapshotDetailProps) {
  return (
    <div className="flex-1 bg-gray-50 overflow-y-auto">
      <div className="p-6 border-b border-gray-200 bg-white">
        <h3 className="text-xl font-semibold text-gray-900 mb-1">快照详情</h3>
        <span className="text-sm text-gray-500">{new Date(snapshot.created_at).toLocaleString()}</span>
      </div>
      
      <div className="p-6 space-y-6">
        <div className="bg-white rounded-2xl p-5 shadow-sm">
          <h4 className="text-base font-semibold text-gray-900 mb-4">存档信息</h4>
          <div className="space-y-3">
            <div>
              <p className="text-sm font-medium text-gray-700 mb-1">原始路径:</p>
              <p className="text-sm text-gray-600 break-all">{snapshot.original_save_path}</p>
            </div>
            <div>
              <p className="text-sm font-medium text-gray-700 mb-1">备份路径:</p>
              <p className="text-sm text-gray-600 break-all">{snapshot.backup_save_path}</p>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-2xl p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-base font-semibold text-gray-900">备注</h4>
            <button
              onClick={onNoteEditModeToggle}
              className="px-4 py-1.5 text-sm font-medium text-blue-600 hover:text-blue-700 hover:bg-blue-50 rounded-lg transition-colors"
            >
              {noteEditMode ? "保存" : "编辑"}
            </button>
          </div>
          {noteEditMode ? (
            <textarea
              value={noteText}
              onChange={(e) => onNoteTextChange(e.target.value)}
              placeholder="记录你对这个存档的备注..."
              className="w-full min-h-[200px] p-4 bg-gray-50 border border-gray-200 rounded-xl text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-y"
            />
          ) : (
            <div className="min-h-[200px] p-4 bg-gray-50 rounded-xl text-sm text-gray-700 whitespace-pre-wrap">
              {noteText || <span className="text-gray-400 italic">点击编辑添加备注...</span>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
