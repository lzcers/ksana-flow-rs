import { type DesignTokens, darkTheme, mergeTheme } from '@incremark/react';

const customTheme: Partial<DesignTokens> = {
  color: {
    brand: {
      primary: '#8b5cf6',
      primaryHover: '#7c3aed',
      primaryActive: '#6d28d9',
      primaryLight: '#ddd6fe'
    },
    neutral: {
      1: '#ffffff',
      2: '#fafafa',
      3: '#f4f4f5',
      4: '#e4e4e7',
      5: '#d4d4d8',
      6: '#a1a1aa',
      7: '#71717a',
      8: '#52525b',
      9: '#3f3f46',
      10: '#27272a'
    },
    text: {
      primary: '#e4e4e7',
      secondary: '#a1a1aa',
      tertiary: '#71717a',
      inverse: '#09090b'
    },
    background: {
      base: '#000000',
      elevated: '#18181b',
      overlay: '#18181b'
    },
    border: {
      subtle: '#27272a',
      default: '#3f3f46',
      strong: '#52525b'
    },
    code: {
      inlineBackground: '#27272a',
      inlineText: '#e4e4e7',
      blockBackground: '#18181b',
      blockText: '#e4e4e7',
      headerBackground: '#27272a'
    },
    status: {
      pending: '#eab308',
      completed: '#22c55e'
    },
    interactive: {
      link: '#3b82f6',
      linkHover: '#2563eb',
      linkVisited: '#8b5cf6',
      checked: '#8b5cf6'
    }
  }
};

export const theme = mergeTheme(darkTheme, customTheme);
