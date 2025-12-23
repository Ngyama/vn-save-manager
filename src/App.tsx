import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Game, Snapshot, Screenshot } from "./types";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import AddGameModal from "./components/AddGameModal";
import GameList from "./components/GameList";
import SnapshotList from "./components/SnapshotList";
import SnapshotDetail from "./components/SnapshotDetail";
import ScreenshotList from "./components/ScreenshotList";
import ScreenshotDetail from "./components/ScreenshotDetail";
import ConfirmDialog from "./components/ConfirmDialog";

function App() {
  const [showWelcome, setShowWelcome] = useState(true);
  const [games, setGames] = useState<Game[]>([]);
  const [selectedGame, setSelectedGame] = useState<Game | null>(null);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<Snapshot | null>(null);
  const [screenshots, setScreenshots] = useState<Screenshot[]>([]);
  const [selectedScreenshot, setSelectedScreenshot] = useState<Screenshot | null>(null);
  const [activeTab, setActiveTab] = useState<"snapshots" | "screenshots">("snapshots");
  const [showAddModal, setShowAddModal] = useState(false);
  const [savePath, setSavePath] = useState<string>("");
  const [exePath, setExePath] = useState<string>("");
  const [noteText, setNoteText] = useState<string>("");
  const [imageCache, setImageCache] = useState<Record<string, string>>({});
  const [noteEditMode, setNoteEditMode] = useState<boolean>(true);
  const [confirmDialog, setConfirmDialog] = useState<{
    show: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  }>({ show: false, title: "", message: "", onConfirm: () => {} });

  useEffect(() => {
    const unlistenSnapshot = listen<Snapshot>("snapshot-created", (event) => {
      console.log("New snapshot!", event.payload);
      if (selectedGame && event.payload.game_id === selectedGame.id) {
        setSnapshots((prev) => [event.payload, ...prev]);
      }
    });

    const unlistenScreenshot = listen<Screenshot>("screenshot-created", (event) => {
      console.log("New screenshot!", event.payload);
      if (selectedGame && event.payload.game_id === selectedGame.id) {
        setScreenshots((prev) => [event.payload, ...prev]);
      }
    });

    return () => {
      unlistenSnapshot.then((f) => f());
      unlistenScreenshot.then((f) => f());
    };
  }, [selectedGame]);

  function handleStart() {
    console.log("Start button clicked, switching to main view");
    setShowWelcome(false);
    loadGames();
  }

  async function loadGames() {
    const gs = await invoke<Game[]>("get_games");
    setGames(gs);
    if (gs.length > 0 && !selectedGame) {
      selectGame(gs[0]);
    }
  }

  async function selectGame(game: Game) {
    setSelectedGame(game);
    const snaps = await invoke<Snapshot[]>("get_snapshots", { gameId: game.id });
    setSnapshots(snaps);
    const screens = await invoke<Screenshot[]>("get_screenshots", { gameId: game.id });
    setScreenshots(screens);
    setSelectedSnapshot(null);
    setSelectedScreenshot(null);
    setNoteText("");
    setImageCache({});
  }

  function selectSnapshot(snapshot: Snapshot) {
    setSelectedSnapshot(snapshot);
    setSelectedScreenshot(null);
    setNoteText(snapshot.note || "");
    setNoteEditMode(true);
  }

  function selectScreenshot(screenshot: Screenshot) {
    setSelectedScreenshot(screenshot);
    setSelectedSnapshot(null);
    setNoteText(screenshot.note || "");
    setNoteEditMode(true);
  }

  useEffect(() => {
    if (!selectedSnapshot && !selectedScreenshot) return;

    const timer = setTimeout(async () => {
      try {
        if (selectedSnapshot) {
          await invoke("update_snapshot_note", {
            snapshotId: selectedSnapshot.id,
            note: noteText,
          });
          setSnapshots((prev) =>
            prev.map((s) =>
              s.id === selectedSnapshot.id ? { ...s, note: noteText } : s
            )
          );
          setSelectedSnapshot((prev) =>
            prev ? { ...prev, note: noteText } : null
          );
        } else if (selectedScreenshot) {
          await invoke("update_screenshot_note", {
            screenshotId: selectedScreenshot.id,
            note: noteText,
          });
          setScreenshots((prev) =>
            prev.map((s) =>
              s.id === selectedScreenshot.id ? { ...s, note: noteText } : s
            )
          );
          setSelectedScreenshot((prev) =>
            prev ? { ...prev, note: noteText } : null
          );
        }
      } catch (e) {
        console.error("Failed to save note:", e);
      }
    }, 1000);

    return () => clearTimeout(timer);
  }, [noteText, selectedSnapshot, selectedScreenshot]);

  useEffect(() => {
    async function preloadScreenshotImages() {
      const missing = screenshots.filter(
        (s) => !imageCache[s.id]
      );
      if (missing.length === 0) return;

      const results = await Promise.all(
        missing.map(async (s) => {
          try {
            const dataUrl = await invoke<string>("load_screenshot_image_base64", {
              imagePath: s.image_path,
            });
            return [s.id, dataUrl] as const;
          } catch (e) {
            console.error("Failed to load screenshot image:", s.id, e);
            return null;
          }
        })
      );

      setImageCache((prev) => {
        const next = { ...prev };
        for (const entry of results) {
          if (entry) {
            const [id, url] = entry;
            next[id] = url;
          }
        }
        return next;
      });
    }

    if (screenshots.length > 0) {
      preloadScreenshotImages();
    }
  }, [screenshots]);

  function handleDeleteGame(game: Game) {
    setConfirmDialog({
      show: true,
      title: "删除游戏",
      message: `确定要删除游戏「${game.name}」以及它的所有快照吗？此操作不可撤销。`,
      onConfirm: async () => {
        setConfirmDialog({ show: false, title: "", message: "", onConfirm: () => {} });
        try {
          await invoke("delete_game", { gameId: game.id });
          if (selectedGame?.id === game.id) {
            setSelectedGame(null);
            setSnapshots([]);
            setSelectedSnapshot(null);
            setNoteText("");
          }
          await loadGames();
        } catch (e) {
          console.error("Failed to delete game:", e);
          alert("删除游戏失败: " + e);
        }
      },
    });
  }

  function handleDeleteSnapshot(snapshot: Snapshot) {
    const snapshotTime = new Date(snapshot.created_at).toLocaleString();
    setConfirmDialog({
      show: true,
      title: "删除快照",
      message: `确定要删除这个快照吗？\n时间：${snapshotTime}\n\n此操作不可撤销。`,
      onConfirm: async () => {
        setConfirmDialog({ show: false, title: "", message: "", onConfirm: () => {} });
        try {
          await invoke("delete_snapshot", { snapshotId: snapshot.id });
          setSnapshots((prev) => prev.filter((s) => s.id !== snapshot.id));
          if (selectedSnapshot?.id === snapshot.id) {
            setSelectedSnapshot(null);
            setNoteText("");
          }
        } catch (e) {
          console.error("Failed to delete snapshot:", e);
          alert("删除快照失败: " + e);
        }
      },
    });
  }

  function handleDeleteScreenshot(screenshot: Screenshot) {
    const screenshotTime = new Date(screenshot.created_at).toLocaleString();
    setConfirmDialog({
      show: true,
      title: "删除截图",
      message: `确定要删除这个截图吗？\n时间：${screenshotTime}\n\n此操作不可撤销。`,
      onConfirm: async () => {
        setConfirmDialog({ show: false, title: "", message: "", onConfirm: () => {} });
        try {
          await invoke("delete_screenshot", { screenshotId: screenshot.id });
          setScreenshots((prev) => prev.filter((s) => s.id !== screenshot.id));
          if (selectedScreenshot?.id === screenshot.id) {
            setSelectedScreenshot(null);
            setNoteText("");
          }
        } catch (e) {
          console.error("Failed to delete screenshot:", e);
          alert("删除截图失败: " + e);
        }
      },
    });
  }

  async function handleAddGame(name: string, saveFolderPath: string, exeFilePath: string) {
    if (!saveFolderPath || saveFolderPath.trim() === "") {
      alert("请先选择存档文件夹");
      return;
    }
    if (!exeFilePath || exeFilePath.trim() === "") {
      alert("请先选择游戏的可执行文件 (.exe)");
      return;
    }
    try {
      console.log("Adding game:", name, saveFolderPath, exeFilePath);
      const result = await invoke("add_game", {
        name,
        saveFolderPath,
        exePath: exeFilePath,
      });
      console.log("Game added successfully:", result);
      await loadGames();
      setShowAddModal(false);
      setSavePath("");
      setExePath("");
    } catch (e) {
      console.error("Failed to add game:", e);
      alert("添加游戏失败: " + e);
    }
  }

  async function handleAddGameClick() {
    setSavePath("");
    setExePath("");
    setShowAddModal(true);
  }

  async function browseSaveFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择存档文件夹",
      });
      if (selected && typeof selected === "string") {
        setSavePath(selected);
      }
    } catch (e) {
      console.error("Failed to select save folder:", e);
    }
  }

  async function browseExeFile() {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: "选择游戏可执行文件 (.exe)",
        filters: [
          {
            name: "Executable",
            extensions: ["exe"],
          },
        ],
      });
      if (selected && typeof selected === "string") {
        setExePath(selected);
      }
    } catch (e) {
      console.error("Failed to select exe file:", e);
    }
  }

  if (showWelcome) {
    console.log("Rendering welcome page");
    return (
      <div className="flex items-center justify-center h-screen w-screen bg-gradient-to-br from-blue-50 to-indigo-50">
        <div className="text-center">
          <h1 className="text-5xl font-semibold text-gray-900 mb-8">Galgame Save Assistant</h1>
          <button
            className="px-12 py-4 bg-blue-500 hover:bg-blue-600 text-white text-xl font-medium rounded-2xl shadow-lg hover:shadow-xl transition-all duration-200 active:scale-95"
            onClick={handleStart}
          >
            Start
          </button>
        </div>
      </div>
    );
  }

  console.log("Rendering main functional page with three columns");
  return (
    <div className="flex h-screen w-screen bg-gray-50">
      <GameList
        games={games}
        selectedGame={selectedGame}
        onSelectGame={selectGame}
        onDeleteGame={handleDeleteGame}
        onAddGame={handleAddGameClick}
      />

      {selectedGame ? (
        <div className="w-96 bg-white border-r border-gray-200 flex flex-col h-screen flex-shrink-0">
          <div className="flex border-b border-gray-200 bg-gray-50/50">
            <button
              className={`flex-1 py-3 px-4 text-sm font-medium transition-colors relative ${
                activeTab === "snapshots"
                  ? "text-blue-600"
                  : "text-gray-500 hover:text-gray-700"
              }`}
              onClick={() => {
                setActiveTab("snapshots");
                setSelectedSnapshot(null);
                setSelectedScreenshot(null);
                setNoteText("");
              }}
            >
              快照
              {activeTab === "snapshots" && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-600"></span>
              )}
            </button>
            <button
              className={`flex-1 py-3 px-4 text-sm font-medium transition-colors relative ${
                activeTab === "screenshots"
                  ? "text-blue-600"
                  : "text-gray-500 hover:text-gray-700"
              }`}
              onClick={() => {
                setActiveTab("screenshots");
                setSelectedSnapshot(null);
                setSelectedScreenshot(null);
                setNoteText("");
              }}
            >
              截图
              {activeTab === "screenshots" && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-600"></span>
              )}
            </button>
          </div>
          {activeTab === "snapshots" ? (
            <SnapshotList
              gameName={selectedGame.name}
              gameFolderPath={selectedGame.game_folder_path}
              snapshots={snapshots}
              selectedSnapshot={selectedSnapshot}
              imageCache={imageCache}
              onSelectSnapshot={selectSnapshot}
              onDeleteSnapshot={handleDeleteSnapshot}
            />
          ) : (
            <ScreenshotList
              gameName={selectedGame.name}
              screenshots={screenshots}
              selectedScreenshot={selectedScreenshot}
              imageCache={imageCache}
              onSelectScreenshot={selectScreenshot}
              onDeleteScreenshot={handleDeleteScreenshot}
            />
          )}
        </div>
      ) : (
        <div className="w-96 bg-white border-r border-gray-200 flex items-center justify-center h-screen">
          <div className="text-gray-400 text-lg">选择或添加游戏开始</div>
        </div>
      )}

      {selectedSnapshot ? (
        <SnapshotDetail
          snapshot={selectedSnapshot}
          imageCache={imageCache}
          noteText={noteText}
          noteEditMode={noteEditMode}
          onNoteTextChange={setNoteText}
          onNoteEditModeToggle={() => setNoteEditMode(!noteEditMode)}
        />
      ) : selectedScreenshot ? (
        <ScreenshotDetail
          screenshot={selectedScreenshot}
          imageCache={imageCache}
          noteText={noteText}
          noteEditMode={noteEditMode}
          onNoteTextChange={setNoteText}
          onNoteEditModeToggle={() => setNoteEditMode(!noteEditMode)}
        />
      ) : (
        <div className="flex-1 bg-gray-50 flex items-center justify-center">
          <div className="text-gray-400 text-base">选择一个快照或截图查看详情</div>
        </div>
      )}

      <AddGameModal
        show={showAddModal}
        savePath={savePath}
        exePath={exePath}
        onClose={() => {
          setShowAddModal(false);
          setSavePath("");
          setExePath("");
        }}
        onSavePathChange={setSavePath}
        onExePathChange={setExePath}
        onBrowseSaveFolder={browseSaveFolder}
        onBrowseExeFile={browseExeFile}
        onSubmit={handleAddGame}
      />

      <ConfirmDialog
        show={confirmDialog.show}
        title={confirmDialog.title}
        message={confirmDialog.message}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog({ show: false, title: "", message: "", onConfirm: () => {} })}
      />
    </div>
  );
}

export default App;
