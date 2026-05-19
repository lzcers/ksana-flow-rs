import React, { useEffect, useMemo, useRef, useState } from 'react';
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
  const [displayText, setDisplayText] = useState('');
  const hasCompletedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  const displayTextRef = useRef('');
  const timerRef = useRef<number | null>(null);
  const incremarkOptions = useMemo(() => ({
    math: { tex: true },
    gfm: true,
  }), []);
  const incremark = useIncremark(incremarkOptions);
  const incremarkRef = useRef(incremark);

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const completeDisplay = () => {
    if (!hasCompletedRef.current) {
      hasCompletedRef.current = true;
      onCompleteRef.current?.();
    }
  };

  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  useEffect(() => {
    incremarkRef.current = incremark;
  }, [incremark]);

  useEffect(() => {
    if (!displayText) {
      incremarkRef.current.reset();
      return;
    }

    incremarkRef.current.render(displayText);
  }, [displayText]);

  useEffect(() => {
    clearTimer();

    if (!text) {
      displayTextRef.current = '';
      setDisplayText('');
      completeDisplay();
      return;
    }

    if (!text.startsWith(displayTextRef.current)) {
      displayTextRef.current = '';
      setDisplayText('');
    }

    if (displayTextRef.current === text) {
      completeDisplay();
      return;
    }

    hasCompletedRef.current = false;
    const tick = () => {
      const current = displayTextRef.current;

      if (!text.startsWith(current)) {
        displayTextRef.current = '';
        setDisplayText('');
        timerRef.current = window.setTimeout(tick, speed);
        return;
      }

      const next = text.slice(0, current.length + 1);
      displayTextRef.current = next;
      setDisplayText(next);

      if (next.length >= text.length) {
        timerRef.current = null;
        completeDisplay();
        return;
      }

      timerRef.current = window.setTimeout(tick, speed);
    };

    timerRef.current = window.setTimeout(tick, speed);

    return () => {
      clearTimer();
    };
  }, [text, speed]);

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
