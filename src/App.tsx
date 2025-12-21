import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Game, Snapshot } from "./types";
import { open } from "@tauri-apps/plugin-dialog";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./App.css";

function App() {
  const [showWelcome, setShowWelcome] = useState(true);
  const [games, setGames] = useState<Game[]>([]);
  const [selectedGame, setSelectedGame] = useState<Game | null>(null);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<Snapshot | null>(null);
  const [showAddModal, setShowAddModal] = useState(false);
  const [savePath, setSavePath] = useState<string>("");
  const [exePath, setExePath] = useState<string>("");
  const [noteText, setNoteText] = useState<string>("");
  const [imageCache, setImageCache] = useState<Record<string, string>>({});
  const [noteEditMode, setNoteEditMode] = useState<boolean>(true);

  useEffect(() => {
    const unlisten = listen<Snapshot>("snapshot-created", (event) => {
      console.log("New snapshot!", event.payload);
      if (selectedGame && event.payload.game_id === selectedGame.id) {
        setSnapshots((prev) => [event.payload, ...prev]);
      }
    });

    return () => {
      unlisten.then((f) => f());
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
    setSelectedSnapshot(null);
    setNoteText("");
    setImageCache({});
  }

  function selectSnapshot(snapshot: Snapshot) {
    setSelectedSnapshot(snapshot);
    setNoteText(snapshot.note || "");
    setNoteEditMode(true);
  }

  useEffect(() => {
    if (!selectedSnapshot) return;

    const timer = setTimeout(async () => {
      try {
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
      } catch (e) {
        console.error("Failed to save note:", e);
      }
    }, 1000);

    return () => clearTimeout(timer);
  }, [noteText, selectedSnapshot]);

  useEffect(() => {
    async function preloadImages() {
      const missing = snapshots.filter(
        (s) => s.image_path && !imageCache[s.id]
      );
      if (missing.length === 0) return;

      const results = await Promise.all(
        missing.map(async (s) => {
          try {
            const dataUrl = await invoke<string>("load_snapshot_image_base64", {
              imagePath: s.image_path,
            });
            return [s.id, dataUrl] as const;
          } catch (e) {
            console.error("Failed to load snapshot image:", s.id, e);
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

    if (snapshots.length > 0) {
      preloadImages();
    }
  }, [snapshots, imageCache]);

  async function handleDeleteGame(game: Game) {
    if (!confirm(`确定要删除游戏「${game.name}」以及它的所有快照吗？`)) {
      return;
    }
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
      <div className="welcome-container">
        <div className="welcome-content">
          <h1 className="welcome-title">Galgame Save Assistant</h1>
          <button className="start-button" onClick={handleStart}>
            Start
          </button>
        </div>
      </div>
    );
    }

  console.log("Rendering main functional page with three columns");
  return (
    <div className="main-container">
      <div className="left-sidebar">
        <h2>My Games</h2>
        <ul className="games-list">
          {games.map((g) => (
            <li
              key={g.id}
              className={selectedGame?.id === g.id ? "active" : ""}
              onClick={() => selectGame(g)}
            >
              <span className="game-name">{g.name}</span>
              <button
                className="game-delete-btn"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteGame(g);
                }}
                title="删除游戏"
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
        <button 
          onClick={handleAddGameClick}
          style={{
            marginTop: "auto",
            padding: "12px 16px",
            backgroundColor: "#646cff",
            color: "white",
            border: "none",
            borderRadius: "8px",
            cursor: "pointer",
            fontSize: "1rem",
            fontWeight: "500",
            width: "100%"
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = "#535bf2";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "#646cff";
          }}
        >
          + Add Game
        </button>
      </div>

      <div className="middle-panel">
        {selectedGame ? (
          <>
            <div className="panel-header">
              <h2>{selectedGame.name}</h2>
              <p className="watching-info">游戏目录: {selectedGame.game_folder_path}</p>
            </div>
            <div className="snapshots-list">
              {snapshots.length === 0 ? (
                <p className="empty-message">No snapshots yet. Save your game to create one!</p>
              ) : (
                snapshots.map((s) => (
                  <div
                    key={s.id}
                    className={`snapshot-item ${selectedSnapshot?.id === s.id ? "selected" : ""}`}
                    onClick={() => selectSnapshot(s)}
                  >
                    {s.image_path && imageCache[s.id] && (
                      <img src={imageCache[s.id]} alt="Snapshot thumbnail" className="snapshot-thumbnail" />
                  )}
                    <div className="snapshot-info">
                      <span className="snapshot-time">{new Date(s.created_at).toLocaleString()}</span>
                      <p className="snapshot-preview">{s.text_content?.substring(0, 50) || "No text captured"}...</p>
                    </div>
                  </div>
                ))
              )}
            </div>
          </>
        ) : (
          <div className="empty-state">Select or Add a Game to start.</div>
        )}
      </div>

      <div className="right-panel">
        {selectedSnapshot ? (
          <>
            <div className="detail-section">
              <h3>Screenshot</h3>
              {selectedSnapshot.image_path && imageCache[selectedSnapshot.id] && (
                <img src={imageCache[selectedSnapshot.id]} alt="Snapshot" className="detail-screenshot" />
              )}
            </div>
            <div className="detail-section">
              <h3>Context</h3>
              <div className="context-content">
                <p className="context-text">{selectedSnapshot.text_content || "No text captured"}</p>
                <p className="context-meta">
                  <strong>Time:</strong> {new Date(selectedSnapshot.created_at).toLocaleString()}
                </p>
                <p className="context-meta">
                  <strong>File:</strong> {selectedSnapshot.original_save_path}
                </p>
              </div>
            </div>
            <div className="detail-section">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px" }}>
                <h3 style={{ margin: 0 }}>Notes</h3>
                <button
                  onClick={() => setNoteEditMode(!noteEditMode)}
                  style={{
                    padding: "6px 12px",
                    fontSize: "0.85rem",
                    backgroundColor: "#646cff",
                    color: "white",
                    border: "none",
                    borderRadius: "4px",
                    cursor: "pointer",
                  }}
                >
                  {noteEditMode ? "Preview" : "Edit"}
                </button>
              </div>
              {noteEditMode ? (
                <textarea
                  className="notes-input"
                  value={noteText}
                  onChange={(e) => setNoteText(e.target.value)}
                  placeholder="Write your notes in Markdown..."
                />
              ) : (
                <div className="notes-preview">
                  {noteText ? (
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {noteText}
                    </ReactMarkdown>
                  ) : (
                    <p style={{ color: "#888", fontStyle: "italic" }}>No notes yet. Click Edit to add notes.</p>
                  )}
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="empty-detail">Select a snapshot to view details</div>
        )}
      </div>

      {showAddModal && (
        <div className="modal">
          <div className="modal-content">
            <h3>添加游戏</h3>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const form = e.target as HTMLFormElement;
                const gameName = form.gameName.value;
                console.log("Form submit - name:", gameName, "save:", savePath, "exe:", exePath);
                if (!savePath) {
                  alert("请先选择存档文件夹");
                  return;
                }
                if (!exePath) {
                  alert("请先选择游戏的可执行文件 (.exe)");
                  return;
                }
                handleAddGame(gameName, savePath, exePath);
              }}
            >
              <label style={{ display: "block", marginBottom: "8px", color: "#888", fontSize: "0.9rem" }}>
                游戏名称
              </label>
              <input name="gameName" placeholder="输入游戏名称" required style={{ marginBottom: "20px" }} />
              
              <label style={{ display: "block", marginBottom: "8px", color: "#888", fontSize: "0.9rem" }}>
                存档文件夹
              </label>
              <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
                <input 
                  name="savePath" 
                  placeholder="请选择存档文件夹" 
                  value={savePath}
                  onChange={(e) => setSavePath(e.target.value)}
                  required 
                  style={{ flex: 1 }}
                  readOnly
                />
                <button
                  type="button"
                  onClick={browseSaveFolder}
                  style={{
                    whiteSpace: "nowrap",
                    backgroundColor: "#2a2a3a",
                    color: "#fff",
                  }}
                >
                  浏览...
                </button>
              </div>

              <label style={{ display: "block", marginBottom: "8px", color: "#888", fontSize: "0.9rem" }}>
                游戏可执行文件 (.exe)
              </label>
              <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
                <input 
                  name="exePath" 
                  placeholder="请选择游戏exe文件" 
                  value={exePath}
                  onChange={(e) => setExePath(e.target.value)}
                  required 
                  style={{ flex: 1 }}
                  readOnly
                />
                <button
                  type="button"
                  onClick={browseExeFile}
                  style={{
                    whiteSpace: "nowrap",
                    backgroundColor: "#2a3a3a",
                    color: "#fff",
                  }}
                >
                  浏览...
                </button>
              </div>
              <p style={{ fontSize: "0.85rem", color: "#888", marginBottom: "20px", marginTop: "-10px" }}>
                提示：请选择游戏的存档文件夹和exe文件。软件会监控存档文件（例如 .dat）的变化，并针对该exe所在窗口生成快照。
              </p>
              
              <div className="actions">
                <button
                  type="button"
                  onClick={() => {
                    setShowAddModal(false);
                    setSavePath("");
                    setExePath("");
                  }}
                  style={{
                    backgroundColor: "#3a3a3a",
                    color: "#fff",
                  }}
                >
                  取消
                </button>
                <button 
                  type="submit"
                  style={{
                    backgroundColor: "#646cff",
                    color: "white",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = "#535bf2";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "#646cff";
                  }}
                >
                  添加
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
