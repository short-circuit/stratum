import '@excalidraw/excalidraw/index.css';
import BoardGallery from './BoardGallery';
import BoardEditor from './BoardEditor';
import { useWhiteboardPanel } from './WhiteboardPanel.shared';

export default function WhiteboardPanelDesktop() {
  const {
    boards,
    activeBoard,
    sceneData,
    libraryItems,
    excalidrawRef,
    dirty,
    loadBoard,
    createBoard,
    deleteBoards,
    handleRename,
    navigateBack,
    handleChange,
    handleLibraryChange,
  } = useWhiteboardPanel();

  if (!activeBoard) {
    return (
      <BoardGallery
        boards={boards}
        onLoadBoard={loadBoard}
        onCreateBoard={createBoard}
        onDeleteBoards={deleteBoards}
        onRename={handleRename}
      />
    );
  }

  return (
    <BoardEditor
      activeBoard={activeBoard}
      sceneData={sceneData}
      libraryItems={libraryItems}
      dirty={dirty}
      excalidrawRef={excalidrawRef}
      onChange={handleChange}
      onLibraryChange={handleLibraryChange}
      onNavigateBack={navigateBack}
    />
  );
}
