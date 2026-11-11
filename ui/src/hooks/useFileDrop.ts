import { useState, useCallback } from "react";

export type DroppedFile = { name: string; path?: string; file?: File };

export function useFileDrop() {
  const [droppedFiles, setDroppedFiles] = useState<DroppedFile[]>([]);
  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const files = Array.from(e.dataTransfer.files).map((f) => ({ name: f.name, file: f }));
    setDroppedFiles((prev) => [...prev, ...files]);
  }, []);
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);
  return { droppedFiles, setDroppedFiles, handleDrop, handleDragOver };
}

export default useFileDrop;