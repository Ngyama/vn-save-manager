import { Screenshot } from "../types";

interface ScreenshotListProps {
  gameName: string;
  screenshots: Screenshot[];
  selectedScreenshot: Screenshot | null;
  imageCache: Record<string, string>;
  onSelectScreenshot: (screenshot: Screenshot) => void;
  onDeleteScreenshot: (screenshot: Screenshot) => void;
}

export default function ScreenshotList({
  gameName,
  screenshots,
  selectedScreenshot,
  imageCache,
  onSelectScreenshot,
  onDeleteScreenshot,
}: ScreenshotListProps) {
  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="p-5 border-b border-gray-200 bg-white">
        <h2 className="text-xl font-semibold text-gray-900 mb-2">{gameName} - 截图</h2>
        <p className="text-sm text-gray-500">按 F11 键截取游戏窗口</p>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {screenshots.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-gray-400 text-base">还没有截图。按 F11 键截取游戏窗口！</p>
          </div>
        ) : (
          screenshots.map((s) => (
            <div
              key={s.id}
              className={`group relative p-4 rounded-xl cursor-pointer transition-all duration-200 ${
                selectedScreenshot?.id === s.id
                  ? "bg-blue-50 border-2 border-blue-200 shadow-sm"
                  : "bg-white border-2 border-transparent hover:border-gray-200 hover:shadow-sm"
              }`}
              onClick={() => onSelectScreenshot(s)}
            >
              <div className="flex gap-4">
                {imageCache[s.id] && (
                  <img
                    src={imageCache[s.id]}
                    alt="Screenshot thumbnail"
                    className="w-32 h-20 object-cover rounded-lg flex-shrink-0"
                  />
                )}
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
