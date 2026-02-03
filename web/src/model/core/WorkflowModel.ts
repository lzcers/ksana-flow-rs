/**
 * WorkflowModel (Core)
 * 纯粹的、同步的、基于 Immer 的状态管理核心。
 * 负责：
 * 1. 持有 WorkflowState
 * 2. 管理 History (Undo/Redo)
 * 3. 执行 Command (调用 Processor)
 */

import type { WorkflowState } from '../types';
import type { GraphCommand } from '../commands';

// Command Handler 类型定义
export type CommandProcessor<T extends GraphCommand = GraphCommand> = (
  state: WorkflowState,
  command: T
) => WorkflowState;

export interface WorkflowModelOptions {
  initialState?: WorkflowState;
  maxHistorySize?: number;
  onStateChange?: (state: WorkflowState) => void;
  onError?: (error: unknown, command: GraphCommand) => void;
}

const defaultState: WorkflowState = {
  nodes: [],
  edges: [],
  selectedNodeId: null,
};

export class WorkflowModel {
  private _state: WorkflowState;
  private _past: WorkflowState[] = [];
  private _future: WorkflowState[] = [];
  private _handlers = new Map<string, CommandProcessor>();
  private _options: WorkflowModelOptions;

  constructor(options: WorkflowModelOptions = {}) {
    this._options = options;
    this._state = options.initialState ?? defaultState;
  }

  // ===== Getters =====

  get state(): WorkflowState {
    return this._state;
  }

  get canUndo(): boolean {
    return this._past.length > 0;
  }

  get canRedo(): boolean {
    return this._future.length > 0;
  }

  // ===== Public API =====

  /**
   * 注册 Command 处理器
   */
  registerHandler(type: string, handler: CommandProcessor): void {
    this._handlers.set(type, handler);
  }

  /**
   * 批量注册处理器
   */
  registerHandlers(handlers: Record<string, CommandProcessor>): void {
    Object.entries(handlers).forEach(([type, handler]) => {
      this._handlers.set(type, handler);
    });
  }

  /**
   * 执行 Command
   */
  execute(command: GraphCommand): void {
    // 1. 处理 Undo/Redo (Meta Commands)
    if (command.type === 'UNDO') {
      this._performUndo();
      return;
    }
    if (command.type === 'REDO') {
      this._performRedo();
      return;
    }

    // 2. 查找 Processor
    const handler = this._handlers.get(command.type);
    if (!handler) {
      console.warn(`[WorkflowModel] No handler for command: ${command.type}`);
      return;
    }

    try {
      // 3. 执行 Processor (Immer produce inside or wrapper)
      // 注意：Processor 本身已经是 Immer producer，或者返回新状态
      // 我们假设 Processor 签名是 (state, command) => nextState
      const nextState = handler(this._state, command);

      // 4. 如果状态未改变，直接返回
      if (nextState === this._state) {
        return;
      }

      // 5. 记录历史 (除非 skipHistory)
      const shouldSkipHistory =
        command.meta?.skipHistory === true ||
        command.type === 'SELECT_NODE' ||
        command.type === 'UPDATE_NODE_STATUS'; // 运行时状态通常不进历史

      if (!shouldSkipHistory) {
        this._pushHistory(this._state);
      }

      // 6. 更新状态
      this._updateState(nextState);

    } catch (error) {
      console.error(`[WorkflowModel] Error executing command ${command.type}:`, error);
      this._options.onError?.(error, command);
    }
  }

  /**
   * 撤销
   */
  undo(): void {
    this.execute({ type: 'UNDO', payload: {} });
  }

  /**
   * 重做
   */
  redo(): void {
    this.execute({ type: 'REDO', payload: {} });
  }

  /**
   * 直接重置状态 (不记录历史，清空历史)
   */
  reset(state: WorkflowState): void {
    this._past = [];
    this._future = [];
    this._updateState(state);
  }

  // ===== Private Implementation =====

  private _updateState(nextState: WorkflowState): void {
    this._state = nextState;
    this._options.onStateChange?.(nextState);
  }

  private _pushHistory(state: WorkflowState): void {
    this._past.push(state);
    const maxSize = this._options.maxHistorySize ?? 50;
    if (this._past.length > maxSize) {
      this._past.shift();
    }
    this._future = []; // Clear redo stack on new action
  }

  private _performUndo(): void {
    if (this._past.length === 0) return;

    const previousState = this._past.pop();
    if (previousState) {
      this._future.push(this._state);
      this._updateState(previousState);
    }
  }

  private _performRedo(): void {
    if (this._future.length === 0) return;

    const nextState = this._future.pop();
    if (nextState) {
      this._past.push(this._state);
      this._updateState(nextState);
    }
  }
}
