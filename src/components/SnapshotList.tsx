import { useState, useMemo } from "react";
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
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isBatchMode, setIsBatchMode] = useState<boolean>(false);
  const [isDeleting, setIsDeleting] = useState<boolean>(false);

  const filteredSnapshots = useMemo(() => {
    if (!searchQuery.trim()) {
      return snapshots;
    }
    const query = searchQuery.toLowerCase();
    return snapshots.filter((s) => 
      s.name.toLowerCase().includes(query) ||
      s.note?.toLowerCase().includes(query) ||
      s.original_save_path.toLowerCase().includes(query) ||
      new Date(s.created_at).toLocaleString().toLowerCase().includes(query)
    );
  }, [snapshots, searchQuery]);

  function handleSelectAll() {
    if (selectedIds.size === filteredSnapshots.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredSnapshots.map(s => s.id)));
    }
  }

  function handleToggleSelect(snapshotId: string) {
    const newSelected = new Set(selectedIds);
    if (newSelected.has(snapshotId)) {
      newSelected.delete(snapshotId);
    } else {
      newSelected.add(snapshotId);
    }
    setSelectedIds(newSelected);
  }

  async function handleBatchDelete() {
    if (selectedIds.size === 0) {
      alert("请选择要删除的快照");
      return;
    }

    if (!confirm(`确定要删除选中的 ${selectedIds.size} 个快照吗？\n\n此操作不可撤销。`)) {
      return;
    }

    setIsDeleting(true);
    try {
      await invoke("batch_delete_snapshots", { snapshotIds: Array.from(selectedIds) });
      setSelectedIds(new Set());
      setIsBatchMode(false);
      if (onSnapshotUpdate) {
        onSnapshotUpdate();
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("批量删除失败: " + errorMsg);
    } finally {
      setIsDeleting(false);
    }
  }

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
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("更新快照名称失败: " + errorMsg);
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
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xl font-semibold text-gray-900">{gameName}</h2>
          <div className="flex items-center gap-4 text-sm">
            <span className="text-gray-600">
              <span className="font-medium text-blue-600">{snapshots.length}</span> 个快照
            </span>
          </div>
        </div>
        <p className="text-sm text-gray-500 mb-3">游戏目录: {gameFolderPath}</p>
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            placeholder="搜索快照..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 px-3 py-2 text-sm bg-gray-50 border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                setIsBatchMode(!isBatchMode);
                setSelectedIds(new Set());
              }}
              className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                isBatchMode
                  ? "bg-blue-600 text-white hover:bg-blue-700"
                  : "bg-gray-100 text-gray-700 hover:bg-gray-200"
              }`}
            >
              {isBatchMode ? "取消批量" : "批量操作"}
            </button>
            {isBatchMode && (
              <>
                <button
                  onClick={handleSelectAll}
                  className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
                >
                  {selectedIds.size === filteredSnapshots.length ? "取消全选" : "全选"}
                </button>
                <button
                  onClick={handleBatchDelete}
                  disabled={selectedIds.size === 0 || isDeleting}
                  className="px-3 py-1.5 text-sm font-medium text-white bg-red-600 hover:bg-red-700 disabled:bg-gray-400 disabled:cursor-not-allowed rounded-lg transition-colors"
                >
                  {isDeleting ? `删除中 (${selectedIds.size})...` : `删除选中 (${selectedIds.size})`}
                </button>
              </>
            )}
          </div>
          {filteredSnapshots.length > 0 && (
            <span className="text-sm text-gray-500">
              {filteredSnapshots.length} / {snapshots.length}
            </span>
          )}
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {filteredSnapshots.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-gray-400 text-base">
              {searchQuery ? "没有找到匹配的快照" : "还没有快照。保存游戏时会自动创建快照！"}
            </p>
          </div>
        ) : (
          filteredSnapshots.map((s) => (
            <div
              key={s.id}
              className={`group relative p-4 rounded-xl transition-all duration-200 ${
                isBatchMode
                  ? selectedIds.has(s.id)
                    ? "bg-blue-50 border-2 border-blue-500 shadow-sm"
                    : "bg-white border-2 border-gray-200 hover:border-gray-300"
                  : selectedSnapshot?.id === s.id
                  ? "bg-blue-50 border-2 border-blue-200 shadow-sm cursor-pointer"
                  : "bg-white border-2 border-transparent hover:border-gray-200 hover:shadow-sm cursor-pointer"
              }`}
              onClick={() => {
                if (isBatchMode) {
                  handleToggleSelect(s.id);
                } else {
                  onSelectSnapshot(s);
                }
              }}
            >
              <div className="flex flex-col gap-2">
                <div className="flex items-center justify-between">
                  {isBatchMode && (
                    <input
                      type="checkbox"
                      checked={selectedIds.has(s.id)}
                      onChange={(e) => {
                        e.stopPropagation();
                        handleToggleSelect(s.id);
                      }}
                      className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500"
                      onClick={(e) => e.stopPropagation()}
                    />
                  )}
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
