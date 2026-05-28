import type { NavigateFunction, NavigateOptions, To } from 'react-router-dom';

let navigator: NavigateFunction | null = null;

export function installNavigator(nextNavigator: NavigateFunction | null) {
  navigator = nextNavigator;
}

export function navigateTo(to: To, options?: NavigateOptions) {
  navigator?.(to, options);
}
