/**
 * Minimal reproduction for https://github.com/erweixin/RaTeX/issues/42
 *
 * RaTeXView intermittently renders blank when mounted inside flex-1/minHeight:0
 * containers — especially inside FlatList items where layout is deferred.
 *
 * The bug: RaTeXView's native side sees 0 available space during the first
 * layout pass, returns early without rendering, and never recovers.
 *
 * Three test cases:
 *   A) flex:1 + minHeight:0 parent (the failing case)
 *   B) Inside a paging FlatList with flex:1 items (deferred layout)
 *   C) Explicit dimensions (always works — control)
 */

import { StatusBar } from "expo-status-bar";
import { useState, useCallback, useLayoutEffect, useRef } from "react";
import {
  StyleSheet,
  Text,
  View,
  Pressable,
  SafeAreaView,
  FlatList,
  useWindowDimensions,
  ScrollView,
} from "react-native";
import { InlineTeX, RaTeXView } from "ratex-react-native";

const EXPR = String.raw`x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`;
const EXPR2 = String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}`;
const EXPR3 = String.raw`\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}`;
const INLINE_FONT_FAMILY = "cofosans-regular";
const INLINE_FONT_CONTENT =
  "For a right triangle, the sides satisfy $a^2 + b^2 = c^2$, where c is the hypotenuse.";

const PAGES = [
  { id: "1", latex: EXPR },
  { id: "2", latex: EXPR2 },
  { id: "3", latex: EXPR3 },
  { id: "4", latex: String.raw`E = mc^2` },
  { id: "5", latex: String.raw`\text{勾股定理：} a^2+b^2=c^2` },
  {
    id: "6",
    latex: String.raw`\ce{CO2 + C -> 2 CO} \quad \text{二氧化碳}`,
  },
  { id: "7", latex: String.raw`\text{😊} \quad E=mc^2` },
];

// ─── Streaming math: already-rendered lines jump on each appended line ──
//
// Simulates an LLM streaming a multi-line formula: one line is appended to a
// growing `\begin{aligned}` block on a timer. On Android (Fabric), the shadow
// node measures the NEW latex synchronously at commit (the box grows), but the
// view swaps its renderer asynchronously a frame later — so for one frame the
// old (shorter) content is re-centered inside the taller box and every
// already-rendered line visibly nudges down, then snaps back.
//
// Repro requirements (deliberate):
//   - The RaTeXView key is STABLE across appends — a per-append key would
//     remount the view and show a blank flash instead of the jump.
//   - The formula is TOP-ALIGNED in its card — a centering parent would move
//     the whole view on growth and mask the intra-view nudge.
const STREAM_LINES = [
  String.raw`f(x) &= (x + 1)^2`,
  String.raw`&= x^2 + 2x + 1`,
  String.raw`\int_0^\infty e^{-x^2}\,dx &= \frac{\sqrt{\pi}}{2}`,
  String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} &= \frac{\pi^2}{6}`,
  String.raw`e^{i\pi} + 1 &= 0`,
  String.raw`\frac{d}{dx}\left(\frac{u}{v}\right) &= \frac{u'v - uv'}{v^2}`,
  String.raw`x &= \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`,
  String.raw`\lim_{x \to 0} \frac{\sin x}{x} &= 1`,
];

function CaseStreamingMath() {
  const [lineCount, setLineCount] = useState(1);
  const [running, setRunning] = useState(true);
  const [autoGrow, setAutoGrow] = useState(false);

  useLayoutEffect(() => {
    if (!running) return;
    const id = setInterval(() => {
      // Wrap around so the jump can be observed continuously.
      setLineCount((n) => (n >= STREAM_LINES.length ? 1 : n + 1));
    }, 700);
    return () => clearInterval(id);
  }, [running]);

  const latex =
    String.raw`\begin{aligned}` +
    STREAM_LINES.slice(0, lineCount).join(String.raw` \\ `) +
    String.raw`\end{aligned}`;

  return (
    <View style={[styles.card, styles.streamCard]}>
      <View style={styles.cardHeader}>
        <Text style={styles.cardTitle}>
          Streaming math — watch rendered lines jump (Android)
        </Text>
      </View>
      <Text style={styles.streamHint}>
        A line is appended every 700 ms ({lineCount}/{STREAM_LINES.length}).
        Broken: on each append the already-rendered lines nudge down for one
        frame, then snap back. Fixed / iOS: lines never move.
      </Text>
      <Pressable
        style={styles.button}
        onPress={() => setRunning((r) => !r)}
      >
        <Text style={styles.buttonText}>{running ? "Pause" : "Resume"}</Text>
      </Pressable>
      <Pressable
        style={styles.streamCheckbox}
        onPress={() => setAutoGrow((a) => !a)}
      >
        <Text style={styles.streamCheckboxText}>
          {autoGrow
            ? "☑ Auto-grow (no fixed height)"
            : "☐ Fixed height (will downscale)"}
        </Text>
      </Pressable>
      <View style={[styles.streamStage, autoGrow && styles.streamStageAuto]}>
        <RaTeXView latex={latex} fontSize={20} displayMode={true} />
      </View>
    </View>
  );
}

// ─── Case #120: useLayoutEffect measurement drives an absolute decoration ─────
// https://github.com/erweixin/RaTeX/issues/120
//
// The dashed frame and blue baseline are positioned absolutely from geometry
// computed *inside useLayoutEffect* via ref.measure(). React flushes layout
// effects (and the state they set) before paint, so:
//   - Fixed:   the view reports its real size on the first commit → the frame
//              hugs the formula the instant it appears (no flash).
//   - Broken:  the view reports 0×0 on the first commit → the frame starts
//              collapsed and only jumps into place a few frames later.
// The caption records what the FIRST useLayoutEffect pass measured, so the
// per-platform behavior is visible even if the jump is quick.
function MeasuredFormula({ latex }: { latex: string }) {
  const hostRef = useRef<View>(null);
  const [deco, setDeco] = useState<{ w: number; h: number } | null>(null);
  const [firstPass, setFirstPass] = useState<{ w: number; h: number } | null>(
    null
  );
  const measure = useCallback(() => {
    const node = hostRef.current;
    if (!node) return;
    node.measure((_x, _y, w, h) => {
      // Capture only the FIRST commit's measurement (the #120 signal).
      setFirstPass((prev) => prev ?? { w, h });
      // Guard object identity so an unchanged size doesn't re-trigger a render.
      setDeco((prev) => (prev && prev.w === w && prev.h === h ? prev : { w, h }));
    });
  }, []);

  // Runs once after the first commit; the component is keyed so remounts re-test.
  useLayoutEffect(() => {
    measure();
  }, [measure]);

  const ok = firstPass ? firstPass.w > 0 && firstPass.h > 0 : null;

  return (
    <View style={styles.measureBody}>
      <View style={styles.measureStage}>
        {/* collapsable={false} keeps the wrapper in the native tree on Android
            so measure() targets it instead of being flattened away. */}
        <View ref={hostRef} collapsable={false} style={styles.measureHost}>
          <RaTeXView
            latex={latex}
            fontSize={30}
            displayMode={true}
            onContentSizeChange={measure}
          />
          {deco && deco.w > 0 && deco.h > 0 ? (
            <>
              <View
                pointerEvents="none"
                style={[styles.decoFrame, { width: deco.w, height: deco.h }]}
              />
              <View
                pointerEvents="none"
                style={[styles.decoBaseline, { width: deco.w, top: deco.h }]}
              />
            </>
          ) : null}
        </View>
      </View>
      <Text style={styles.measureMeta}>
        {"useLayoutEffect measure(): "}
        {firstPass
          ? `${firstPass.w.toFixed(0)}×${firstPass.h.toFixed(0)}`
          : "—"}
        {ok === null
          ? ""
          : ok
          ? "  ✓ real size on first commit"
          : "  ✗ 0×0 on first commit (flash)"}
      </Text>
    </View>
  );
}

function CaseLayoutEffectMeasure() {
  const [remount, setRemount] = useState(0);
  const [idx, setIdx] = useState(0);
  const latex = idx === 0 ? EXPR : idx === 1 ? EXPR2 : EXPR3;

  return (
    <View style={[styles.card, styles.measureCard]}>
      <View style={styles.cardHeader}>
        <Text style={styles.cardTitle}>
          #120: useLayoutEffect measure → absolute decoration
        </Text>
      </View>
      <Text style={styles.measureHint}>
        The dashed frame + blue baseline are sized from ref.measure() inside
        useLayoutEffect. Fixed → they hug the formula immediately. Broken → they
        start at 0 and jump. Press Remount to re-test the first commit.
      </Text>
      <MeasuredFormula key={`${remount}-${idx}`} latex={latex} />
      <View style={styles.smokeRow}>
        <Pressable style={styles.smokeBtn} onPress={() => setRemount((r) => r + 1)}>
          <Text style={styles.smokeBtnText}>Remount</Text>
        </Pressable>
        <Pressable style={styles.smokeBtn} onPress={() => setIdx((i) => (i + 1) % 3)}>
          <Text style={styles.smokeBtnText}>Next formula</Text>
        </Pressable>
      </View>
    </View>
  );
}

// ─── Detach-cancel repro: transient detach permanently blanks formulas ──
//
// RaTeXView launches its render as a coroutine and cancels it in
// onDetachedFromWindow. On Android a view can be detached *transiently* and
// come right back — most commonly when a Fabric commit changes an ancestor's
// VIEW-FLATTENING status: the ancestor is removed and its children re-parented
// (SurfaceMountingManager.removeViewAt + addView), firing detach/attach on
// every descendant. Nothing ever restarts the cancelled job — every trigger
// is a prop setter guarded by `if (field == value) return`, and the props
// never change — so the view keeps its laid-out size but stays blank forever,
// reporting the confusing onError "StandaloneCoroutine was cancelled".
//
// This case reproduces it the way a real markdown/math consumer hits it:
//   1. each formula mounts via the two-pass measure→swap pattern (invisible
//      probe → synchronous measure() in useLayoutEffect → pre-paint swap to
//      a fitted view), inside a wrapper styled `opacity: 0` — an overlay
//      hiding an extension until its frame is known;
//   2. one frame later the wrapper is revealed by DROPPING the opacity
//      style. Losing it makes the wrapper flattenable, Fabric restructures,
//      and every formula view is detached while its render job is still in
//      flight (the first job also blocks on the KaTeX font load on a fresh
//      process, keeping the rest pending).
//   Broken: formulas occupy space but stay blank — red dot, 0/32 rendered,
//           "StandaloneCoroutine was cancelled" listed below the strip.
//   Fixed:  onAttachedToWindow re-kicks the render — all 32 appear.
// Cold start (kill + relaunch) is the most reliable; "Run repro" re-tests
// warm.
const DETACH_COUNT = 32;
const DETACH_FORMULAS = Array.from({ length: DETACH_COUNT }, (_, i) =>
  String.raw`\begin{aligned} \sum_{k=${i}}^{\infty} \frac{${i + 1}}{k^{${(i % 3) + 2}}} &= \int_0^\infty \frac{x^{${i}}}{e^x - 1}\,dx \\ \frac{-b \pm \sqrt{b^2 - 4a_{${i}}c}}{2a_{${i}}} &= \prod_{k=1}^{${i + 2}} \left(1 + \frac{x_k}{k!}\right) \end{aligned}`
);

function TwoPassFormula({
  index,
  latex,
  onSized,
  onErrorMsg,
}: {
  index: number;
  latex: string;
  onSized: (index: number) => void;
  onErrorMsg: (message: string) => void;
}) {
  const probeRef = useRef<View>(null);
  const [fit, setFit] = useState<{ w: number; h: number } | null>(null);

  // Fabric's measure() is synchronous — the probe reports its real size in
  // the mounting commit, and the sync setState swaps in the fitted view
  // before paint.
  useLayoutEffect(() => {
    if (fit) return;
    probeRef.current?.measure((_x, _y, w, h) => {
      if (w > 0 && h > 0) setFit({ w, h });
    });
  }, [fit]);

  if (!fit) {
    return (
      <View>
        <View style={{ position: "absolute", width: 10000, height: 10000, opacity: 0 }}>
          <View ref={probeRef} collapsable={false} style={{ alignSelf: "flex-start" }}>
            <RaTeXView latex={latex} fontSize={13} displayMode={true} />
          </View>
        </View>
      </View>
    );
  }

  const scale = Math.min(1, 136 / fit.w, 52 / fit.h);
  return (
    <RaTeXView
      latex={latex}
      fontSize={13}
      displayMode={true}
      style={{ width: fit.w * scale, height: fit.h * scale }}
      onContentSizeChange={(e) => {
        const { width, height } = e.nativeEvent;
        console.log(`[detach-${index}] size: ${width}x${height}`);
        if (width > 0 && height > 0) onSized(index);
      }}
      onError={(e) => {
        console.error(`[detach-${index}] error:`, e.nativeEvent.error);
        onErrorMsg(`#${index}: ${e.nativeEvent.error}`);
      }}
    />
  );
}

function CaseDetachCancel() {
  const [gen, setGen] = useState(0);
  const [chunks, setChunks] = useState(0);
  const [sized, setSized] = useState<Record<number, true>>({});
  const [errors, setErrors] = useState<string[]>([]);

  const run = useCallback(() => {
    setSized({});
    setErrors([]);
    setChunks(0);
    setGen((g) => g + 1);
  }, []);

  // Overlay-style reveal churn: each formula mounts inside a wrapper View
  // styled `opacity: 0` (the way a markdown overlay hides an extension until
  // its frame is measured), and one frame later the wrapper is revealed by
  // DROPPING that style. Losing `opacity` makes the wrapper eligible for
  // Fabric view flattening, so the commit restructures the native tree by
  // REMOVING and re-inserting the wrapper — transiently detaching the
  // formula view while its render job is still in flight.
  const [revealed, setRevealed] = useState(false);
  useLayoutEffect(() => {
    setRevealed(false);
    let count = 0;
    let raf = requestAnimationFrame(function tick() {
      setChunks((c) => c + 1);
      count += 1;
      if (count === 1) setRevealed(true);
      if (count < 3) raf = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(raf);
  }, [gen]);

  const onSized = useCallback((index: number) => {
    setSized((s) => (s[index] ? s : { ...s, [index]: true }));
  }, []);
  const onErrorMsg = useCallback((message: string) => {
    setErrors((errs) => [...errs, message]);
  }, []);

  const renderedCount = Object.keys(sized).length;
  const status: "pending" | "ok" | "error" =
    errors.length > 0
      ? "error"
      : renderedCount === DETACH_COUNT
      ? "ok"
      : "pending";

  return (
    <View style={[styles.card, styles.detachCard]}>
      <View style={styles.cardHeader}>
        <StatusDot status={status} />
        <Text style={styles.cardTitle}>
          Detach-cancel: clipped views stay blank (Android)
        </Text>
      </View>
      <Text style={styles.smokeHint}>
        {`Each formula mounts inside an opacity:0 wrapper that is revealed one frame later — the style drop flips the wrapper's view-flattening status, so Fabric re-parents it and transiently detaches every formula mid-render. Broken: they stay blank forever ("StandaloneCoroutine was cancelled" below). Fixed: all ${DETACH_COUNT} render. Kill + relaunch the app for the most reliable (cold-font) run.`}
      </Text>
      <View style={styles.smokeRow}>
        <Pressable style={styles.smokeBtn} onPress={run}>
          <Text style={styles.smokeBtnText}>Run repro</Text>
        </Pressable>
        <Text style={styles.detachMeta}>
          rendered {renderedCount}/{DETACH_COUNT} · errors {errors.length}
        </Text>
      </View>
      <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.detachStrip}>
        {Array.from({ length: chunks }, (_, c) => (
          <View key={`chunk-${gen}-${c}`} style={styles.detachChunk}>
            <Text style={styles.detachChunkText}>chunk {c}</Text>
          </View>
        ))}
        {DETACH_FORMULAS.map((latex, i) => (
          <View key={`${gen}-${i}`} style={styles.detachBox}>
            <View style={revealed ? undefined : { opacity: 0 }}>
              <TwoPassFormula index={i} latex={latex} onSized={onSized} onErrorMsg={onErrorMsg} />
            </View>
            <Text style={styles.detachIndex}>#{i}</Text>
          </View>
        ))}
      </ScrollView>
      {errors.length > 0 ? (
        <Text style={styles.detachErrors} numberOfLines={3}>
          {errors.join("  ·  ")}
        </Text>
      ) : null}
    </View>
  );
}

// ─── Status indicator ────────────────────────────────────────────────
function StatusDot({ status }: { status: "pending" | "ok" | "error" }) {
  const color =
    status === "ok" ? "#22c55e" : status === "error" ? "#ef4444" : "#d1d5db";
  return <View style={[styles.dot, { backgroundColor: color }]} />;
}

// ─── Case A: flex-1 + minHeight:0 parent (the bug trigger) ──────────
function CaseFlexMinHeight({ renderKey }: { renderKey: number }) {
  const [status, setStatus] = useState<"pending" | "ok" | "error">("pending");

  return (
    <View style={styles.card}>
      <View style={styles.cardHeader}>
        <StatusDot status={status} />
        <Text style={styles.cardTitle}>
          A: flex:1 + minHeight:0 parent
        </Text>
      </View>
      {/* This is the layout pattern that causes intermittent failures */}
      <View style={{ flex: 1, minHeight: 0 }}>
        <View style={{ flex: 1, minHeight: 0, alignItems: "center" }}>
          <RaTeXView
            // key={renderKey}
            latex={EXPR}
            fontSize={28}
            displayMode={true}
            onContentSizeChange={(e) => {
              const { width, height } = e.nativeEvent;
              console.log(`[A] size: ${width}x${height}`);
              if (width > 0 && height > 0) setStatus("ok");
            }}
            onError={(e) => {
              console.error(`[A] error:`, e.nativeEvent.error);
              setStatus("error");
            }}
          />
        </View>
      </View>
    </View>
  );
}

// ─── Case B: Inside paging FlatList (deferred layout) ───────────────
function CaseFlatList({ renderKey }: { renderKey: number }) {
  const { width } = useWindowDimensions();
  const [statuses, setStatuses] = useState<Record<string, "pending" | "ok" | "error">>({});

  const renderPage = useCallback(
    ({ item }: { item: (typeof PAGES)[0] }) => (
      <View style={[styles.flatListPage, { width }]}>
        {/* Mimics a card inside a FlatList page with flex constraints */}
        <View style={{ flex: 1, minHeight: 0, justifyContent: "center", alignItems: "center" }}>
          <RaTeXView
            // key={`${renderKey}-${item.id}`}
            latex={item.latex}
            fontSize={28}
            displayMode={true}
            onContentSizeChange={(e) => {
              const { width: w, height: h } = e.nativeEvent;
              console.log(`[B-${item.id}] size: ${w}x${h}`);
              if (w > 0 && h > 0)
                setStatuses((s) => ({ ...s, [item.id]: "ok" }));
            }}
            onError={(e) => {
              console.error(`[B-${item.id}] error:`, e.nativeEvent.error);
              setStatuses((s) => ({ ...s, [item.id]: "error" }));
            }}
          />
        </View>
      </View>
    ),
    [renderKey, width]
  );

  const allOk = PAGES.every((p) => statuses[p.id] === "ok");
  const anyError = PAGES.some((p) => statuses[p.id] === "error");

  return (
    <View style={styles.card}>
      <View style={styles.cardHeader}>
        <StatusDot status={anyError ? "error" : allOk ? "ok" : "pending"} />
        <Text style={styles.cardTitle}>B: Paging FlatList + flex:1</Text>
      </View>
      <FlatList
        // key={renderKey}
        data={PAGES}
        renderItem={renderPage}
        // keyExtractor={(item) => `${renderKey}-${item.id}`}
        horizontal
        pagingEnabled
        style={{ flex: 1, minHeight: 0 }}
        showsHorizontalScrollIndicator={false}
      />
    </View>
  );
}

// ─── PR #45 smoke: auto size, fixed box scale-down, prop churn, callback identity ─
function CasePR45Smoke() {
  const [latexIdx, setLatexIdx] = useState(0);
  const [displayMode, setDisplayMode] = useState(true);
  /** Bump to give `onContentSizeChange` a new function identity (Paper / emitter edge cases). */
  const [handlerGen, setHandlerGen] = useState(0);
  const [last, setLast] = useState<{ w: number; h: number } | null>(null);
  const [evtCount, setEvtCount] = useState(0);
  const [fixedOk, setFixedOk] = useState<"pending" | "ok" | "error">("pending");

  const onContentSizeChange = useCallback(
    (e: { nativeEvent: { width: number; height: number } }) => {
      setLast({ w: e.nativeEvent.width, h: e.nativeEvent.height });
      setEvtCount((c) => c + 1);
    },
    [handlerGen]
  );

  return (
    <View style={[styles.card, styles.smokeCard]}>
      <View style={styles.cardHeader}>
        <StatusDot status={last && last.w > 0 && last.h > 0 ? "ok" : "pending"} />
        <Text style={styles.cardTitle}>PR #45 smoke (auto + scale-down)</Text>
      </View>
      <Text style={styles.smokeMeta}>
        Events #{evtCount} · intrinsic size{" "}
        {last ? `${last.w.toFixed(0)}×${last.h.toFixed(0)}` : "—"} · displayMode=
        {displayMode ? "block" : "inline"}
      </Text>
      <View style={styles.smokeRow}>
        <Pressable
          style={styles.smokeBtn}
          onPress={() => setLatexIdx((i) => (i + 1) % 3)}
        >
          <Text style={styles.smokeBtnText}>Next formula</Text>
        </Pressable>
        <Pressable
          style={styles.smokeBtn}
          onPress={() => setDisplayMode((d) => !d)}
        >
          <Text style={styles.smokeBtnText}>Toggle displayMode</Text>
        </Pressable>
        <Pressable
          style={styles.smokeBtn}
          onPress={() => setHandlerGen((g) => g + 1)}
        >
          <Text style={styles.smokeBtnText}>New callback ref</Text>
        </Pressable>
      </View>
      <Text style={styles.smokeHint}>
        {`After changing formula or mode, the counters above should update; after "New callback ref", you should still receive sizes (Paper / Fabric listener fix).`}
      </Text>
      <View style={styles.smokeAutoHost}>
        <RaTeXView
          latex={
            latexIdx === 0 ? EXPR : latexIdx === 1 ? EXPR2 : EXPR3
          }
          fontSize={22}
          displayMode={displayMode}
          onContentSizeChange={onContentSizeChange}
        />
      </View>
      <View style={styles.cardHeader}>
        <StatusDot status={fixedOk} />
        <Text style={styles.cardTitle}>Fixed 100×34 (scale down, no overflow)</Text>
      </View>
      <View style={styles.smokeFixedHost}>
        <RaTeXView
          latex={EXPR2}
          fontSize={26}
          displayMode={true}
          style={{ width: 100, height: 34 }}
          onContentSizeChange={(e) => {
            const { width, height } = e.nativeEvent;
            if (width > 0 && height > 0) setFixedOk("ok");
          }}
          onError={() => setFixedOk("error")}
        />
      </View>
    </View>
  );
}

// ─── InlineTeX custom font smoke: Expo prebuild assets/font(s) path ──
function CaseInlineTeXCustomFont() {
  return (
    <View style={[styles.card, styles.fontCard]}>
      <View style={styles.cardHeader}>
        <Text style={styles.cardTitle}>InlineTeX custom font</Text>
      </View>
      <View style={styles.fontCaseBody}>
        <Text style={styles.fontCaseLabel}>React Native Text</Text>
        <Text style={styles.customFontText}>{INLINE_FONT_CONTENT}</Text>

        <Text style={styles.fontCaseLabel}>InlineTeX textStyle</Text>
        <InlineTeX
          content={INLINE_FONT_CONTENT}
          textStyle={styles.customFontText}
        />
      </View>
    </View>
  );
}

// ─── Case C: Explicit dimensions (control — should always work) ─────
function CaseExplicit({ renderKey }: { renderKey: number }) {
  const [status, setStatus] = useState<"pending" | "ok" | "error">("pending");

  return (
    <View style={styles.card}>
      <View style={styles.cardHeader}>
        <StatusDot status={status} />
        <Text style={styles.cardTitle}>C: Explicit 300x60 (control)</Text>
      </View>
      <View style={{ alignItems: "center", padding: 12 }}>
        <RaTeXView
          // key={renderKey}
          latex={EXPR}
          fontSize={28}
          displayMode={true}
          style={{ width: 200, height: 60 }}
          onContentSizeChange={(e) => {
            const { width, height } = e.nativeEvent;
            console.log(`[C] size: ${width}x${height}`);
            if (width > 0 && height > 0) setStatus("ok");
          }}
          onError={(e) => {
            console.error(`[C] error:`, e.nativeEvent.error);
            setStatus("error");
          }}
        />
      </View>
    </View>
  );
}

// ─── App ─────────────────────────────────────────────────────────────
export default function App() {
  const [key, setKey] = useState(0);

  return (
    <SafeAreaView style={styles.container}>
      <StatusBar style="dark" />
      <ScrollView
        style={styles.scroll}
        contentContainerStyle={styles.scrollContent}
        keyboardShouldPersistTaps="handled"
      >
        <Text style={styles.title}>RaTeX Repro — Issues #120 / #42</Text>
        <Text style={styles.subtitle}>
          Render #{key} — gray dot = pending, green = rendered, red = error
        </Text>

        <Pressable style={styles.button} onPress={() => setKey((k) => k + 1)}>
          <Text style={styles.buttonText}>Force Re-mount (key={key + 1})</Text>
        </Pressable>

        <CaseStreamingMath />
        <CaseDetachCancel />
        <CaseLayoutEffectMeasure />
        <CasePR45Smoke />
        <CaseInlineTeXCustomFont />

        <View style={styles.cases}>
          <CaseFlexMinHeight renderKey={key} />
          <CaseFlatList renderKey={key} />
          <CaseExplicit renderKey={key} />
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: "#f8f9fa",
  },
  title: {
    fontSize: 20,
    fontWeight: "700",
    textAlign: "center",
    marginTop: 12,
  },
  subtitle: {
    fontSize: 12,
    color: "#666",
    textAlign: "center",
    marginTop: 2,
    marginBottom: 8,
  },
  button: {
    backgroundColor: "#007AFF",
    paddingHorizontal: 20,
    paddingVertical: 8,
    borderRadius: 8,
    alignSelf: "center",
    marginBottom: 8,
  },
  buttonText: {
    color: "#fff",
    fontWeight: "600",
    fontSize: 14,
  },
  scroll: { flex: 1 },
  scrollContent: { paddingBottom: 24 },
  cases: {
    flexGrow: 1,
    minHeight: 520,
    padding: 12,
    gap: 12,
  },
  measureCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
    paddingBottom: 12,
  },
  measureHint: {
    fontSize: 10,
    color: "#6b7280",
    paddingHorizontal: 12,
    marginTop: 8,
    marginBottom: 4,
  },
  measureBody: {
    paddingHorizontal: 12,
  },
  measureStage: {
    alignItems: "center",
    justifyContent: "center",
    paddingVertical: 24,
  },
  measureHost: {
    position: "relative",
  },
  decoFrame: {
    position: "absolute",
    left: 0,
    top: 0,
    borderWidth: 2,
    borderColor: "#e11d48",
    borderStyle: "dashed",
    borderRadius: 2,
  },
  decoBaseline: {
    position: "absolute",
    left: 0,
    height: 2,
    backgroundColor: "#2563eb",
  },
  measureMeta: {
    fontSize: 12,
    fontWeight: "600",
    color: "#111827",
    textAlign: "center",
    marginTop: 4,
  },
  smokeCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
    paddingBottom: 12,
  },
  streamCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
    paddingBottom: 12,
  },
  streamHint: {
    fontSize: 11,
    color: "#4b5563",
    paddingHorizontal: 12,
    marginBottom: 8,
  },
  streamStage: {
    // Fixed height + top/left alignment: the card never resizes and the
    // formula's top-left corner never moves, so the only thing that CAN move
    // is the content inside the RaTeXView — the artifact under test.
    // Deliberately shorter than the full 8-line block (~363dp at fontSize 20):
    // the last appends also exercise the clamped case, where the content must
    // scale to fit its container uniformly and without flicker.
    height: 340,
    alignItems: "flex-start",
    justifyContent: "flex-start",
    paddingHorizontal: 12,
    overflow: "hidden",
  },
  streamStageAuto: {
    // No fixed height: the stage grows with the content, exercising the
    // auto-sizing streaming path (no clamp, no scale-to-fit).
    height: "auto",
  },
  streamCheckbox: {
    paddingHorizontal: 12,
    marginBottom: 8,
  },
  streamCheckboxText: {
    fontSize: 13,
    color: "#111827",
  },
  detachCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
    paddingBottom: 12,
  },
  detachMeta: {
    fontSize: 11,
    color: "#4b5563",
    alignSelf: "center",
  },
  detachStrip: {
    marginHorizontal: 12,
    marginTop: 4,
  },
  detachBox: {
    width: 148,
    height: 76,
    marginRight: 8,
    borderRadius: 6,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: "#d1d5db",
    backgroundColor: "#f9fafb",
    alignItems: "center",
    justifyContent: "center",
  },
  detachFormula: {
    width: 136,
    height: 52,
  },
  detachIndex: {
    position: "absolute",
    top: 2,
    right: 5,
    fontSize: 9,
    color: "#9ca3af",
  },
  detachChunk: {
    width: 64,
    height: 76,
    marginRight: 8,
    borderRadius: 6,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: "#bfdbfe",
    backgroundColor: "#eff6ff",
    alignItems: "center",
    justifyContent: "center",
  },
  detachChunkText: {
    fontSize: 10,
    color: "#3b82f6",
  },
  detachErrors: {
    fontSize: 10,
    color: "#ef4444",
    paddingHorizontal: 12,
    marginTop: 6,
  },
  smokeMeta: {
    fontSize: 11,
    color: "#4b5563",
    paddingHorizontal: 12,
    marginBottom: 6,
  },
  smokeHint: {
    fontSize: 10,
    color: "#6b7280",
    paddingHorizontal: 12,
    marginBottom: 8,
  },
  smokeRow: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: 8,
    paddingHorizontal: 12,
    marginBottom: 8,
  },
  smokeBtn: {
    backgroundColor: "#0d9488",
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 6,
  },
  smokeBtnText: {
    color: "#fff",
    fontSize: 12,
    fontWeight: "600",
  },
  smokeAutoHost: {
    alignItems: "center",
    paddingVertical: 8,
    minHeight: 56,
  },
  smokeFixedHost: {
    alignItems: "center",
    paddingBottom: 8,
  },
  fontCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
  },
  fontCaseBody: {
    gap: 8,
    padding: 12,
  },
  fontCaseLabel: {
    color: "#4b5563",
    fontSize: 11,
    fontWeight: "600",
    textTransform: "uppercase",
  },
  customFontText: {
    color: "#111827",
    fontFamily: INLINE_FONT_FAMILY,
    fontSize: 17,
  },
  card: {
    flex: 1,
    backgroundColor: "#fff",
    borderRadius: 12,
    overflow: "hidden",
    shadowColor: "#000",
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.1,
    shadowRadius: 3,
    elevation: 2,
  },
  cardHeader: {
    flexDirection: "row",
    alignItems: "center",
    padding: 10,
    gap: 8,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: "#e5e7eb",
  },
  cardTitle: {
    fontSize: 13,
    fontWeight: "600",
    color: "#374151",
  },
  dot: {
    width: 10,
    height: 10,
    borderRadius: 5,
  },
  flatListPage: {
    flex: 1,
    justifyContent: "center",
    alignItems: "center",
    padding: 16,
  },
});
