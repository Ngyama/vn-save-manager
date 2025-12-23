import { Screenshot } from "../types";

interface ScreenshotDetailProps {
  screenshot: Screenshot;
  imageCache: Record<string, string>;
  noteText: string;
  noteEditMode: boolean;
  onNoteTextChange: (text: string) => void;
  onNoteEditModeToggle: () => void;
}

export default function ScreenshotDetail({
  screenshot,
  imageCache,
  noteText,
  noteEditMode,
  onNoteTextChange,
  onNoteEditModeToggle,
}: ScreenshotDetailProps) {
  const imageUrl = imageCache[screenshot.id];

  return (
    <div className="flex-1 bg-gray-50 overflow-y-auto">
      <div className="p-6 border-b border-gray-200 bg-white">
        <h3 className="text-xl font-semibold text-gray-900 mb-1">截图详情</h3>
        <span className="text-sm text-gray-500">{new Date(screenshot.created_at).toLocaleString()}</span>
      </div>
      
      <div className="p-6 space-y-6">
        {imageUrl && (
          <div className="bg-white rounded-2xl p-4 shadow-sm">
            <img
              src={imageUrl}
              alt="Screenshot"
              className="w-full rounded-xl object-contain bg-gray-100"
            />
          </div>
        )}
        
        <div className="bg-white rounded-2xl p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-base font-semibold text-gray-900">感想</h4>
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
              placeholder="记录你对这个场景的感想..."
              className="w-full min-h-[200px] p-4 bg-gray-50 border border-gray-200 rounded-xl text-sm text-gray-900 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-y"
            />
          ) : (
            <div className="min-h-[200px] p-4 bg-gray-50 rounded-xl text-sm text-gray-700 whitespace-pre-wrap">
              {noteText || <span className="text-gray-400 italic">点击编辑添加感想...</span>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
