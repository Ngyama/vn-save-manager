import { Game } from "../types";

interface GameListProps {
  games: Game[];
  selectedGame: Game | null;
  onSelectGame: (game: Game) => void;
  onDeleteGame: (game: Game) => void;
  onAddGame: () => void;
}

export default function GameList({
  games,
  selectedGame,
  onSelectGame,
  onDeleteGame,
  onAddGame,
}: GameListProps) {
  return (
    <div className="w-64 bg-white border-r border-gray-200 flex flex-col h-screen">
      <div className="p-6 border-b border-gray-100">
        <h2 className="text-2xl font-semibold text-gray-900">我的游戏</h2>
      </div>
      
      <ul className="flex-1 overflow-y-auto p-3 space-y-1.5">
        {games.map((g) => (
          <li
            key={g.id}
            className={`group relative flex items-center justify-between p-3 rounded-xl cursor-pointer transition-all duration-200 ${
              selectedGame?.id === g.id
                ? "bg-blue-50 shadow-sm"
                : "hover:bg-gray-50"
            }`}
            onClick={() => onSelectGame(g)}
          >
            <span className={`flex-1 truncate text-base font-medium ${
              selectedGame?.id === g.id ? "text-blue-600" : "text-gray-900"
            }`}>
              {g.name}
            </span>
            <button
              className={`opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 ${
                selectedGame?.id === g.id ? "text-gray-400" : ""
              }`}
              onClick={(e) => {
                e.stopPropagation();
                onDeleteGame(g);
              }}
              title="删除游戏"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </li>
        ))}
      </ul>
      
      <div className="p-4 border-t border-gray-100">
        <button
          onClick={onAddGame}
          className="w-full py-3 px-4 bg-blue-500 hover:bg-blue-600 text-white font-medium rounded-xl shadow-sm hover:shadow-md transition-all duration-200 active:scale-[0.98]"
        >
          + 添加游戏
        </button>
      </div>
    </div>
  );
}
