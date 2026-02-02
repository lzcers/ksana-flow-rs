/**
 * 新的 Store 入口文件
 * 演示如何使用 RxCommandBus 和 createCanvasNew
 */

import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import type { StoreState } from './types';
import { createWorkflow } from './createWorkflow';
import { createCanvasNew } from './createCanvasNew';
import { createExecution } from './createExecution';
import { createToast } from './createToast';
import { RxCommandBus, registerAllHandlers } from '@/model';
import { connectRxToZustand } from './rxConnector';

// 创建单例 CommandBus
const commandBus = new RxCommandBus({
  enableLogging: process.env.NODE_ENV === 'development',
});

// 注册所有处理器
registerAllHandlers(commandBus);

// 创建 Store
export const useStore = create<StoreState>()(
  devtools(
    immer((set, get, api) => {
      // 组合所有子 store
      const store = {
        ...createWorkflow(set, get, api),
        ...createCanvasNew(set, get, api),
        ...createExecution(set, get, api),
        ...createToast(set, get, api),
      };

      // 初始化 CommandBus 连接
      if (typeof window !== 'undefined') {
        // 延迟初始化，确保 store 已创建
        setTimeout(() => {
          const unsubscribe = connectRxToZustand(api as any, {
            commandBus,
            onStateChange: (state) => {
              console.log('[Store] State updated:', state);
            },
          });

          // 初始化 Canvas 的 CommandBus
          store.initializeCommandBus?.(commandBus);

          // 保存 unsubscribe 函数
          (store as any)._unsubscribeRx = unsubscribe;
        }, 0);
      }

      return store as StoreState;
    }),
    {
      name: 'KasanaFlow Store',
      enabled: process.env.NODE_ENV === 'development',
    }
  )
);

// 导出 CommandBus 供外部使用
export { commandBus };

// 便捷 hook 导出
export * from './hooks';

// 类型导出
export type { StoreState };
