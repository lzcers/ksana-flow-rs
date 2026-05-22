import React, { useEffect, useMemo, useRef } from 'react';
import {
  Incremark,
  darkTheme,
  ThemeProvider,
  useIncremark,
} from '@incremark/react';
import '@incremark/theme/styles.css';

interface TypewriterProps {
  text: string;
  animate?: boolean;
  isFinished?: boolean;
  onComplete?: () => void;
}

const Typewriter: React.FC<TypewriterProps> = ({
  text,
  animate = true,
  isFinished = false,
  onComplete,
}) => {
  const hasCompletedRef = useRef(false);
  const onCompleteRef = useRef(onComplete);
  const previousTextRef = useRef('');
  const incremarkOptions = useMemo(() => ({
    math: { tex: true },
    gfm: true,
    typewriter: {
      enabled: animate,
      effect: 'typing' as const,
    },
  }), [animate]);
  const incremark = useIncremark(incremarkOptions);
  const {
    append,
    finalize,
    isDisplayComplete,
    render,
    reset,
    typewriter: { setEnabled },
  } = incremark;

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
    setEnabled(animate);
  }, [animate, setEnabled]);

  useEffect(() => {
    const previousText = previousTextRef.current;

    if (!text) {
      previousTextRef.current = '';
      reset();
      completeDisplay();
      return;
    }

    if (!animate) {
      previousTextRef.current = text;
      render(text);
      completeDisplay();
      return;
    }

    hasCompletedRef.current = false;

    if (!text.startsWith(previousText)) {
      reset();
      append(text);
    } else if (text.length > previousText.length) {
      append(text.slice(previousText.length));
    }

    if (isFinished) {
      finalize();
    }

    previousTextRef.current = text;
  }, [animate, append, finalize, isFinished, render, reset, text]);

  useEffect(() => {
    if (!text || !animate || !isFinished || !isDisplayComplete) {
      return;
    }

    completeDisplay();
  }, [animate, isDisplayComplete, isFinished, text]);

  return (
    <div className="h-full">
      <ThemeProvider theme={darkTheme}>
        <Incremark incremark={incremark} />
      </ThemeProvider>
    </div>
  );
};

const MemoizedTypewriter = React.memo(Typewriter);

MemoizedTypewriter.displayName = 'Typewriter';

export default MemoizedTypewriter;
