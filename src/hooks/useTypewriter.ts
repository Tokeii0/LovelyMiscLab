import { useEffect, useRef, useState } from "react";

/** The last line that finished typing: coming back to the view shows it whole
 * instead of typing it out again. */
let lastFinished = "";

/** Reveal `text` one character (code point — emoji stay intact) at a time.
 * Returns the revealed slice, whether it finished, and a `skip()` to jump to the
 * end (e.g. on click). Resets whenever `text` changes. */
export function useTypewriter(text: string, speed = 22) {
  const [shown, setShown] = useState(() => (text === lastFinished ? text : ""));
  const [done, setDone] = useState(() => !text || text === lastFinished);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    if (timer.current) window.clearInterval(timer.current);
    if (!text || text === lastFinished) {
      setShown(text);
      setDone(true);
      return;
    }
    setShown("");
    setDone(false);
    const chars = Array.from(text);
    let i = 0;
    timer.current = window.setInterval(() => {
      i += 1;
      setShown(chars.slice(0, i).join(""));
      if (i >= chars.length) {
        if (timer.current) window.clearInterval(timer.current);
        lastFinished = text;
        setDone(true);
      }
    }, speed);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [text, speed]);

  const skip = () => {
    if (timer.current) window.clearInterval(timer.current);
    lastFinished = text;
    setShown(text);
    setDone(true);
  };

  return { shown, done, skip };
}
