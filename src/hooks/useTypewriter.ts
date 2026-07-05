import { useEffect, useRef, useState } from "react";

/** Reveal `text` one character at a time. Returns the revealed slice, whether it
 * finished, and a `skip()` to jump to the end (e.g. on click). Resets whenever
 * `text` changes. */
export function useTypewriter(text: string, speed = 22) {
  const [shown, setShown] = useState("");
  const [done, setDone] = useState(false);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    if (timer.current) window.clearInterval(timer.current);
    setShown("");
    setDone(false);
    if (!text) {
      setDone(true);
      return;
    }
    let i = 0;
    timer.current = window.setInterval(() => {
      i += 1;
      setShown(text.slice(0, i));
      if (i >= text.length) {
        if (timer.current) window.clearInterval(timer.current);
        setDone(true);
      }
    }, speed);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [text, speed]);

  const skip = () => {
    if (timer.current) window.clearInterval(timer.current);
    setShown(text);
    setDone(true);
  };

  return { shown, done, skip };
}
