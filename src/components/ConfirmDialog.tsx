import { useState, useEffect } from "react";

interface ConfirmDialogProps {
  show: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  showDeleteVisualLogger?: boolean;
  onDeleteVisualLoggerChange?: (value: boolean) => void;
  deleteVisualLoggerDefault?: boolean;
}

export default function ConfirmDialog({
  show,
  title,
  message,
  confirmText = "确定",
  cancelText = "取消",
  onConfirm,
  onCancel,
  showDeleteVisualLogger = false,
  onDeleteVisualLoggerChange,
  deleteVisualLoggerDefault = false,
}: ConfirmDialogProps) {
  const [deleteVisualLogger, setDeleteVisualLogger] = useState(deleteVisualLoggerDefault);

  useEffect(() => {
    setDeleteVisualLogger(deleteVisualLoggerDefault);
  }, [deleteVisualLoggerDefault, show]);

  if (!show) return null;

  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.checked;
    setDeleteVisualLogger(value);
    if (onDeleteVisualLoggerChange) {
      onDeleteVisualLoggerChange(value);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        <div className="p-6 border-b border-gray-200">
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
        </div>
        
        <div className="p-6">
          <p className="text-gray-700 leading-relaxed whitespace-pre-line mb-6">
            {message}
          </p>
          
          {showDeleteVisualLogger && (
            <div className="mb-6 flex items-center">
              <input
                type="checkbox"
                id="deleteVisualLogger"
                checked={deleteVisualLogger}
                onChange={handleCheckboxChange}
                className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
              />
              <label htmlFor="deleteVisualLogger" className="ml-2 text-sm text-gray-700">
                同时删除 visual-logger 文件夹
              </label>
            </div>
          )}
          
          <div className="flex justify-end gap-3">
            <button
              type="button"
              onClick={onCancel}
              className="px-6 py-2.5 bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium rounded-xl transition-colors"
            >
              {cancelText}
            </button>
            <button
              type="button"
              onClick={onConfirm}
              className="px-6 py-2.5 bg-red-500 hover:bg-red-600 text-white font-medium rounded-xl shadow-sm hover:shadow-md transition-all"
            >
              {confirmText}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
