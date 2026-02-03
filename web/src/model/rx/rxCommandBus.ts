/**
 * RxCommandBus - Command 总线
 * 使用 RxJS Subject 分发 Command，由注册的处理器处理
 */

import { Subject, Observable, of, from, BehaviorSubject } from 'rxjs';
import { filter, concatMap, catchError, map } from 'rxjs/operators';
import type { WorkflowState } from '../types';
import type { GraphCommand } from '../commands';
import { RxWorkflowState } from './rxState';

// 处理器函数类型
export type CommandHandler<T extends GraphCommand> = (
  state: WorkflowState,
  command: T
) => WorkflowState;

export type AsyncCommandHandler<T extends GraphCommand> = (
  state: WorkflowState,
  command: T
) => Promise<WorkflowState>;

export interface HistoryState {
  past: WorkflowState[];
  future: WorkflowState[];
  canUndo: boolean;
  canRedo: boolean;
}

export interface RxCommandBusOptions {
  initialState?: WorkflowState;
  enableLogging?: boolean;
  maxHistorySize?: number;
  onAfterCommand?: (args: {
    command: GraphCommand;
    prevState: WorkflowState;
    nextState: WorkflowState;
  }) => void;
  onCommandError?: (args: {
    command: GraphCommand;
    error: unknown;
    state: WorkflowState;
  }) => void;
}

export class RxCommandBus {
  private _commands$ = new Subject<GraphCommand>();
  private _state: RxWorkflowState;
  private _handlers = new Map<
    string,
    CommandHandler<any> | AsyncCommandHandler<any>
  >();
  private _options: RxCommandBusOptions;

  // History 栈
  private _past: WorkflowState[] = [];
  private _future: WorkflowState[] = [];
  private _history$ = new BehaviorSubject<HistoryState>({
    past: [],
    future: [],
    canUndo: false,
    canRedo: false,
  });

  // 公共 Observable
  public readonly commands$ = this._commands$.asObservable();
  public get state$(): Observable<WorkflowState> {
    return this._state.state$;
  }
  public get history$(): Observable<HistoryState> {
    return this._history$.asObservable();
  }
  public get canUndo$(): Observable<boolean> {
    return this.history$.pipe(map(h => h.canUndo));
  }
  public get canRedo$(): Observable<boolean> {
    return this.history$.pipe(map(h => h.canRedo));
  }

  constructor(options: RxCommandBusOptions = {}) {
    this._options = options;

    const initialState: WorkflowState = options.initialState ?? {
      nodes: [],
      edges: [],
      selectedNodeId: null,
    };

    this._state = new RxWorkflowState({ initialState });

    // 订阅 commands 流
    this._setupCommandProcessing();
  }

  // ===== 公共方法 =====

  /**
   * 分发 Command
   */
  dispatch(command: GraphCommand): void {
    if (this._options.enableLogging) {
      console.log('[RxCommandBus] Dispatch:', command.type, command.payload);
    }
    this._commands$.next(command);
  }

  /**
   * 撤销
   */
  undo(): void {
    this.dispatch({ type: 'UNDO', payload: {} });
  }

  /**
   * 重做
   */
  redo(): void {
    this.dispatch({ type: 'REDO', payload: {} });
  }

  /**
   * 获取当前状态快照
   */
  get currentState(): WorkflowState {
    return this._state.state;
  }

  /**
   * 直接设置状态（用于初始化或恢复）
   */
  setState(state: WorkflowState): void {
    this._state.next(state);
  }

  /**
   * 注册 Command 处理器
   */
  registerHandler<T extends GraphCommand>(
    type: T['type'],
    handler: CommandHandler<T> | AsyncCommandHandler<T>
  ): void {
    this._handlers.set(type, handler);
  }

  /**
   * 批量注册处理器
   */
  registerHandlers(
    handlers: Record<
      string,
      CommandHandler<any> | AsyncCommandHandler<any>
    >
  ): void {
    Object.entries(handlers).forEach(([type, handler]) => {
      this._handlers.set(type, handler);
    });
  }

  /**
   * 订阅特定类型的 Command
   */
  onCommand<T extends GraphCommand>(type: T['type']): Observable<T> {
    return this._commands$.pipe(
      filter((cmd): cmd is T => cmd.type === type)
    );
  }

  /**
   * 订阅状态变化
   */
  onStateChange(): Observable<WorkflowState> {
    return this._state.state$;
  }

  /**
   * 选择特定节点
   */
  selectNode$(nodeId: string) {
    return this._state.selectNode$(nodeId);
  }

  /**
   * 销毁（清理订阅）
   */
  destroy(): void {
    this._commands$.complete();
    this._history$.complete();
    this._state.destroy();
  }

  // ===== 私有方法 =====

  /**
   * 更新 History 状态流
   */
  private _updateHistoryState(): void {
    this._history$.next({
      past: this._past,
      future: this._future,
      canUndo: this._past.length > 0,
      canRedo: this._future.length > 0,
    });
  }

  /**
   * 设置 Command 处理流程
   */
  private _setupCommandProcessing(): void {
    this._commands$
      .pipe(
        concatMap((command) => {
          const prevState = this.currentState;

          // 特殊处理 UNDO/REDO
          if (command.type === 'UNDO') {
            if (this._past.length > 0) {
              const stateToRestore = this._past[this._past.length - 1];
              this._future.push(prevState);
              this._past.pop();
              this._updateHistoryState();
              return of({ command, prevState, nextState: stateToRestore });
            }
            return of({ command, prevState, nextState: prevState });
          }

          if (command.type === 'REDO') {
            if (this._future.length > 0) {
              const stateToRestore = this._future[this._future.length - 1];
              this._past.push(prevState);
              this._future.pop();
              this._updateHistoryState();
              return of({ command, prevState, nextState: stateToRestore });
            }
            return of({ command, prevState, nextState: prevState });
          }

          const handler = this._handlers.get(command.type);

          if (!handler) {
            console.warn(
              `[RxCommandBus] No handler for command type: ${command.type}`
            );
            return of({ command, prevState, nextState: prevState });
          }

          try {
            const result = handler(prevState, command);
            return from(Promise.resolve(result)).pipe(
              catchError((error) => {
                console.error('[RxCommandBus] Handler error:', error);
                this._options.onCommandError?.({ command, error, state: prevState });
                return of(prevState);
              }),
              concatMap((nextState) => of({ command, prevState, nextState }))
            );
          } catch (error) {
            console.error('[RxCommandBus] Handler error:', error);
            this._options.onCommandError?.({ command, error, state: prevState });
            return of({ command, prevState, nextState: prevState });
          }
        }),
        catchError((error) => {
          console.error('[RxCommandBus] Processing error:', error);
          return of({
            command: { type: '__UNKNOWN__', payload: {} } as any,
            prevState: this.currentState,
            nextState: this.currentState,
          });
        })
      )
      .subscribe(({ command, prevState, nextState }) => {
        if (nextState !== this.currentState) {
          // 处理 History 记录逻辑
          const isHistoryControl = command.type === 'UNDO' || command.type === 'REDO';
          const shouldSkipHistory = command.meta?.skipHistory === true || 
                                  command.type === 'SELECT_NODE' ||
                                  command.type === 'UPDATE_NODE_STATUS';

          if (!isHistoryControl && !shouldSkipHistory) {
            // 记录到 past
            this._past.push(prevState);
            if (this._past.length > (this._options.maxHistorySize ?? 50)) {
              this._past.shift();
            }
            // 产生新历史，清空 future
            this._future = [];
            this._updateHistoryState();
          }

          this._state.next(nextState);
        }
        this._options.onAfterCommand?.({ command, prevState, nextState });
      });
  }
}
