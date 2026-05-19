import React, { useEffect, useMemo, useRef } from 'react';
import {
  AutoScrollContainer,
  darkTheme,
  Incremark,
  ThemeProvider,
  useIncremark,
} from '@incremark/react';
import '@incremark/theme/styles.css';

interface TypewriterProps {
  text: string;
  speed?: number;
  onComplete?: () => void;
}

const Typewriter: React.FC<TypewriterProps> = ({ text, speed = 30, onComplete }) => {
  const hasCompletedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  const incremarkOptions = useMemo(() => ({
    math: { tex: true },
    gfm: true,
    typewriter: {
      enabled: true,
      charsPerTick: 1 as const,
      tickInterval: speed,
      effect: 'none' as const,
    },
  }), [speed]);
  const incremark = useIncremark(incremarkOptions);
  const incremarkRef = useRef(incremark);

  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  useEffect(() => {
    incremarkRef.current = incremark;
  }, [incremark]);

  useEffect(() => {
    hasCompletedRef.current = false;
    incremarkRef.current.render(text);
  }, [text]);

  useEffect(() => {
    if (incremark.isDisplayComplete && !hasCompletedRef.current) {
      hasCompletedRef.current = true;
      onCompleteRef.current?.();
    }
  }, [incremark.isDisplayComplete]);

  return (
    <div className="h-full">
      <ThemeProvider theme={darkTheme}>
        <AutoScrollContainer enabled={false} className="h-full w-full">
          <div className="h-full text-inherit">
            <Incremark incremark={incremark} />
          </div>
        </AutoScrollContainer>
      </ThemeProvider>
    </div>
  );
};

export default Typewriter;
