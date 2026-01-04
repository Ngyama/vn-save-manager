import { useState } from "react";
import { Snapshot } from "../types";
import { invoke } from "@tauri-apps/api/core";

interface SnapshotListProps {
  gameName: string;
  gameFolderPath: string;
  snapshots: Snapshot[];
  selectedSnapshot: Snapshot | null;
  imageCache?: Record<string, string>;
  onSelectSnapshot: (snapshot: Snapshot) => void;
  onDeleteSnapshot: (snapshot: Snapshot) => void;
  onSnapshotUpdate?: () => void;
}

export default function SnapshotList({
  gameName,
  gameFolderPath,
  snapshots,
  selectedSnapshot,
  onSelectSnapshot,
  onDeleteSnapshot,
  onSnapshotUpdate,
}: SnapshotListProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState<string>("");

  async function handleNameEditStart(snapshot: Snapshot) {
    setEditingId(snapshot.id);
    setEditingName(snapshot.name);
  }

  async function handleNameSave(snapshot: Snapshot) {
    if (editingName.trim() === "") {
      setEditingName(snapshot.name);
      setEditingId(null);
      return;
    }

    try {
      await invoke("update_snapshot_name", {
        snapshotId: snapshot.id,
        name: editingName.trim(),
      });
      setEditingId(null);
      if (onSnapshotUpdate) {
        onSnapshotUpdate();
      }
    } catch (e) {
      console.error("Failed to update snapshot name:", e);
      alert("更新快照名称失败: " + e);
      setEditingName(snapshot.name);
      setEditingId(null);
    }
  }

  function handleNameCancel() {
    setEditingId(null);
    setEditingName("");
  }
  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="p-5 border-b border-gray-200 bg-white">
        <h2 className="text-xl font-semibold text-gray-900 mb-2">{gameName}</h2>
        <p className="text-sm text-gray-500">游戏目录: {gameFolderPath}</p>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {snapshots.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-gray-400 text-base">还没有快照。保存游戏时会自动创建快照！</p>
          </div>
        ) : (
          snapshots.map((s) => (
            <div
              key={s.id}
              className={`group relative p-4 rounded-xl cursor-pointer transition-all duration-200 ${
                selectedSnapshot?.id === s.id
                  ? "bg-blue-50 border-2 border-blue-200 shadow-sm"
                  : "bg-white border-2 border-transparent hover:border-gray-200 hover:shadow-sm"
              }`}
              onClick={() => onSelectSnapshot(s)}
            >
              <div className="flex flex-col gap-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-gray-500">
                    {new Date(s.created_at).toLocaleString()}
                  </span>
                  <button
                    className="opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteSnapshot(s);
                    }}
                    title="删除快照"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
                {editingId === s.id ? (
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      value={editingName}
                      onChange={(e) => setEditingName(e.target.value)}
                      onBlur={() => handleNameSave(s)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          handleNameSave(s);
                        } else if (e.key === "Escape") {
                          handleNameCancel();
                        }
                      }}
                      autoFocus
                      className="flex-1 px-3 py-1.5 text-sm text-gray-900 bg-white border border-blue-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      onClick={(e) => e.stopPropagation()}
                    />
                  </div>
                ) : (
                  <div
                    className="text-sm font-medium text-gray-900 cursor-text hover:text-blue-600 transition-colors"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleNameEditStart(s);
                    }}
                    title="点击编辑名称"
                  >
                    {s.name}
                  </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
