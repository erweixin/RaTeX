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
import { useState, useCallback } from "react";
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
import { RaTeXView } from "ratex-react-native";

const EXPR = String.raw`x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`;
const EXPR2 = String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}`;
const EXPR3 = String.raw`\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}`;

const PAGES = [
  { id: "1", latex: EXPR },
  { id: "2", latex: EXPR2 },
  { id: "3", latex: EXPR3 },
  { id: "4", latex: String.raw`E = mc^2` },
];

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
        <Text style={styles.title}>RaTeX Repro — Issue #42</Text>
        <Text style={styles.subtitle}>
          Render #{key} — gray dot = pending, green = rendered, red = error
        </Text>

        <Pressable style={styles.button} onPress={() => setKey((k) => k + 1)}>
          <Text style={styles.buttonText}>Force Re-mount (key={key + 1})</Text>
        </Pressable>

        <CasePR45Smoke />

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
  smokeCard: {
    flex: 0,
    marginHorizontal: 12,
    marginTop: 8,
    marginBottom: 8,
    paddingBottom: 12,
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
