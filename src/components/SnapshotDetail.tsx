import { useState } from "react";
import { Snapshot } from "../types";
import { invoke } from "@tauri-apps/api/core";

interface SnapshotDetailProps {
  snapshot: Snapshot;
  imageCache?: Record<string, string>;
  noteText: string;
  noteEditMode: boolean;
  onNoteTextChange: (text: string) => void;
  onNoteEditModeToggle: () => void;
  onRestoreSuccess?: () => void;
}

export default function SnapshotDetail({
  snapshot,
  noteText,
  noteEditMode,
  onNoteTextChange,
  onNoteEditModeToggle,
  onRestoreSuccess,
}: SnapshotDetailProps) {
  const [isRestoring, setIsRestoring] = useState(false);

  async function handleRestore() {
    if (!confirm(`确定要恢复这个快照吗？\n这将会替换当前的存档文件：\n${snapshot.original_save_path}\n\n此操作不可撤销。`)) {
      return;
    }

    setIsRestoring(true);
    try {
      await invoke("restore_snapshot", { snapshotId: snapshot.id });
      alert("快照恢复成功！");
      if (onRestoreSuccess) {
        onRestoreSuccess();
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("恢复快照失败: " + errorMsg);
    } finally {
      setIsRestoring(false);
    }
  }
  return (
    <div className="flex-1 bg-gray-50 overflow-y-auto">
      <div className="p-6 border-b border-gray-200 bg-white">
        <h3 className="text-xl font-semibold text-gray-900 mb-1">快照详情</h3>
        <span className="text-sm text-gray-500">{new Date(snapshot.created_at).toLocaleString()}</span>
      </div>
      
      <div className="p-6 space-y-6">
        <div className="bg-white rounded-2xl p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-base font-semibold text-gray-900">存档信息</h4>
            <button
              onClick={handleRestore}
              disabled={isRestoring}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors"
            >
              {isRestoring ? "恢复中..." : "恢复快照"}
            </button>
          </div>
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
