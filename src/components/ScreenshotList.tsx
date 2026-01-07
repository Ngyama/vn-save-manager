import { useState, useMemo } from "react";
import { Screenshot } from "../types";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface ScreenshotListProps {
  gameName: string;
  screenshots: Screenshot[];
  selectedScreenshot: Screenshot | null;
  imageCache: Record<string, string>;
  onSelectScreenshot: (screenshot: Screenshot) => void;
  onDeleteScreenshot: (screenshot: Screenshot) => void;
  onScreenshotUpdate?: () => void;
}

export default function ScreenshotList({
  gameName,
  screenshots,
  selectedScreenshot,
  imageCache,
  onSelectScreenshot,
  onDeleteScreenshot,
  onScreenshotUpdate,
}: ScreenshotListProps) {
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isBatchMode, setIsBatchMode] = useState<boolean>(false);
  const [isDeleting, setIsDeleting] = useState<boolean>(false);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState<string>("");

  const filteredScreenshots = useMemo(() => {
    if (!searchQuery.trim()) {
      return screenshots;
    }
    const query = searchQuery.toLowerCase();
    return screenshots.filter((s) => 
      s.name.toLowerCase().includes(query) ||
      s.note?.toLowerCase().includes(query) ||
      s.image_path.toLowerCase().includes(query) ||
      new Date(s.created_at).toLocaleString().toLowerCase().includes(query)
    );
  }, [screenshots, searchQuery]);

  async function handleNameEditStart(screenshot: Screenshot) {
    setEditingId(screenshot.id);
    setEditingName(screenshot.name);
  }

  async function handleNameSave(screenshot: Screenshot) {
    if (editingName.trim() === "") {
      setEditingName(screenshot.name);
      setEditingId(null);
      return;
    }

    try {
      await invoke("update_screenshot_name", {
        screenshotId: screenshot.id,
        name: editingName.trim(),
      });
      setEditingId(null);
      if (onScreenshotUpdate) {
        onScreenshotUpdate();
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("更新截图名称失败: " + errorMsg);
      setEditingName(screenshot.name);
      setEditingId(null);
    }
  }

  function handleNameCancel() {
    setEditingId(null);
    setEditingName("");
  }

  function handleSelectAll() {
    if (selectedIds.size === filteredScreenshots.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredScreenshots.map(s => s.id)));
    }
  }

  function handleToggleSelect(screenshotId: string) {
    const newSelected = new Set(selectedIds);
    if (newSelected.has(screenshotId)) {
      newSelected.delete(screenshotId);
    } else {
      newSelected.add(screenshotId);
    }
    setSelectedIds(newSelected);
  }

  async function handleBatchDelete() {
    if (selectedIds.size === 0) {
      alert("请选择要删除的截图");
      return;
    }

    if (!confirm(`确定要删除选中的 ${selectedIds.size} 张截图吗？\n\n此操作不可撤销。`)) {
      return;
    }

    setIsDeleting(true);
    try {
      await invoke("batch_delete_screenshots", { screenshotIds: Array.from(selectedIds) });
      setSelectedIds(new Set());
      setIsBatchMode(false);
      if (onScreenshotUpdate) {
        onScreenshotUpdate();
      }
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("批量删除失败: " + errorMsg);
    } finally {
      setIsDeleting(false);
    }
  }

  async function handleBatchExport() {
    if (selectedIds.size === 0) {
      alert("请选择要导出的截图");
      return;
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择导出目录",
      });

      if (!selected || typeof selected !== "string") {
        return;
      }

      setIsExporting(true);
      const exportedCount = await invoke<number>("batch_export_screenshots", {
        screenshotIds: Array.from(selectedIds),
        exportDir: selected,
      });

      alert(`成功导出 ${exportedCount} 张截图到:\n${selected}`);
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      alert("批量导出失败: " + errorMsg);
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="p-5 border-b border-gray-200 bg-white">
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-xl font-semibold text-gray-900">{gameName} - 截图</h2>
          <div className="flex items-center gap-4 text-sm">
            <span className="text-gray-600">
              <span className="font-medium text-blue-600">{screenshots.length}</span> 张截图
            </span>
          </div>
        </div>
        <p className="text-sm text-gray-500 mb-3">按 F11 键截取游戏窗口</p>
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            placeholder="搜索截图..."
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
                  {selectedIds.size === filteredScreenshots.length ? "取消全选" : "全选"}
                </button>
                <button
                  onClick={handleBatchExport}
                  disabled={selectedIds.size === 0 || isExporting}
                  className="px-3 py-1.5 text-sm font-medium text-white bg-green-600 hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed rounded-lg transition-colors"
                >
                  {isExporting ? `导出中 (${selectedIds.size})...` : `导出选中 (${selectedIds.size})`}
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
          {filteredScreenshots.length > 0 && (
            <span className="text-sm text-gray-500">
              {filteredScreenshots.length} / {screenshots.length}
            </span>
          )}
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {filteredScreenshots.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-gray-400 text-base">
              {searchQuery ? "没有找到匹配的截图" : "还没有截图。按 F11 键截取游戏窗口！"}
            </p>
          </div>
        ) : (
          filteredScreenshots.map((s) => (
            <div
              key={s.id}
              className={`group relative p-4 rounded-xl transition-all duration-200 ${
                isBatchMode
                  ? selectedIds.has(s.id)
                    ? "bg-blue-50 border-2 border-blue-500 shadow-sm"
                    : "bg-white border-2 border-gray-200 hover:border-gray-300"
                  : selectedScreenshot?.id === s.id
                  ? "bg-blue-50 border-2 border-blue-200 shadow-sm cursor-pointer"
                  : "bg-white border-2 border-transparent hover:border-gray-200 hover:shadow-sm cursor-pointer"
              }`}
              onClick={() => {
                if (isBatchMode) {
                  handleToggleSelect(s.id);
                } else {
                  onSelectScreenshot(s);
                }
              }}
            >
              <div className="flex gap-4">
                {isBatchMode && (
                  <input
                    type="checkbox"
                    checked={selectedIds.has(s.id)}
                    onChange={(e) => {
                      e.stopPropagation();
                      handleToggleSelect(s.id);
                    }}
                    className="w-4 h-4 text-blue-600 rounded focus:ring-blue-500 self-start mt-1"
                    onClick={(e) => e.stopPropagation()}
                  />
                )}
                <div className="w-32 h-20 flex-shrink-0 relative">
                  {imageCache[s.id] ? (
                    <img
                      src={imageCache[s.id]}
                      alt="Screenshot thumbnail"
                      className="w-32 h-20 object-cover rounded-lg"
                    />
                  ) : (
                    <div className="w-32 h-20 bg-gray-200 rounded-lg flex items-center justify-center text-gray-400 text-xs">
                      未加载
                    </div>
                  )}
                </div>
                <div className="flex-1 flex flex-col gap-2 min-w-0">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-500">
                      {new Date(s.created_at).toLocaleString()}
                    </span>
                    <button
                      className="opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteScreenshot(s);
                      }}
                      title="删除截图"
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
                  <p className="text-sm text-gray-600 line-clamp-2">
                    {s.note?.substring(0, 50) || "点击编辑感想"}...
                  </p>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
