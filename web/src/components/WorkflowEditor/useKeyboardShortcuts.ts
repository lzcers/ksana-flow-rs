import React from 'react';
import type { Node, Edge } from '../../model/workflow/types';

interface UseKeyboardShortcutsProps {
  getNodes: () => Node[];
  getEdges: () => Edge[];
  onPaste?: (nodes: Node[], edges: Edge[]) => void;
  onSave?: () => Promise<void>;
  onUndo?: () => void;
  onRedo?: () => void;
  onNodesChange: (changes: any) => void;
  screenToFlowPosition: (position: { x: number; y: number }) => { x: number; y: number };
  mousePositionRef: React.MutableRefObject<{ x: number; y: number }>;
}

export const useKeyboardShortcuts = ({
  getNodes,
  getEdges,
  onPaste,
  onSave,
  onUndo,
  onRedo,
  onNodesChange,
  screenToFlowPosition,
  mousePositionRef,
}: UseKeyboardShortcutsProps) => {
  React.useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      const activeElement = document.activeElement;
      const isInputActive = activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement;
      const key = e.key.toLowerCase();

      if ((e.ctrlKey || e.metaKey) && key === 'z' && !e.shiftKey) {
        if (isInputActive) return;
        e.preventDefault();
        onUndo?.();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (key === 'y' || (key === 'z' && e.shiftKey))) {
        if (isInputActive) return;
        e.preventDefault();
        onRedo?.();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && key === 's') {
        e.preventDefault();
        await onSave?.();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && key === 'c') {
        if (isInputActive) return;

        const selectedNodes = getNodes().filter(n => n.selected);
        if (selectedNodes.length === 0) return;

        const selectedNodeIds = new Set(selectedNodes.map(n => n.id));
        const selectedEdges = getEdges().filter(e =>
          selectedNodeIds.has(e.source) && selectedNodeIds.has(e.target)
        );

        const data = {
          nodes: selectedNodes,
          edges: selectedEdges
        };
        await navigator.clipboard.writeText(JSON.stringify(data));
      }

      if ((e.ctrlKey || e.metaKey) && key === 'x') {
        if (isInputActive) return;

        const selectedNodes = getNodes().filter(n => n.selected);
        if (selectedNodes.length === 0) return;

        const selectedNodeIds = new Set(selectedNodes.map(n => n.id));
        const selectedEdges = getEdges().filter(e =>
          selectedNodeIds.has(e.source) && selectedNodeIds.has(e.target)
        );

        const data = {
          nodes: selectedNodes,
          edges: selectedEdges
        };
        await navigator.clipboard.writeText(JSON.stringify(data));

        onNodesChange(selectedNodes.map(n => ({ type: 'remove', id: n.id })));
      }

      if ((e.ctrlKey || e.metaKey) && key === 'v') {
        if (isInputActive) return;
        try {
          const text = await navigator.clipboard.readText();
          const data = JSON.parse(text);
          if (!data.nodes || !Array.isArray(data.nodes)) return;

          const nodes = data.nodes;
          const minX = Math.min(...nodes.map((n: any) => n.position.x));
          const minY = Math.min(...nodes.map((n: any) => n.position.y));
          const maxX = Math.max(...nodes.map((n: any) => n.position.x + (n.measured?.width || n.width || 0)));
          const maxY = Math.max(...nodes.map((n: any) => n.position.y + (n.measured?.height || n.height || 0)));
          const centerX = (minX + maxX) / 2;
          const centerY = (minY + maxY) / 2;

          const targetScreen = mousePositionRef.current;
          const targetPos = screenToFlowPosition(targetScreen);

          const offsetX = targetPos.x - centerX;
          const offsetY = targetPos.y - centerY;

          const newNodes = nodes.map((n: any) => ({
            ...n,
            position: {
              x: n.position.x + offsetX,
              y: n.position.y + offsetY
            }
          }));

          onPaste?.(newNodes, data.edges || []);
        } catch {
          // Ignore invalid JSON or clipboard issues
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getNodes, getEdges, onPaste, screenToFlowPosition, onSave, onUndo, onRedo, onNodesChange]);
};
