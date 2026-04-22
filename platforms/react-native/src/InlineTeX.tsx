import React, {useMemo, useState} from 'react';
import {StyleSheet, Text, View} from 'react-native';
import type {ColorValue, StyleProp, TextStyle} from 'react-native';
import {RaTeXView} from './RaTeXView';

type Segment =
  | {type: 'text'; content: string}
  | {type: 'latex'; id: number; content: string};

function parseInlineTeX(text: string): Segment[] {
  const result: Segment[] = [];
  const parts = text.split(/(\$[^$]+\$)/g);
  let latexId = 0;
  for (const part of parts) {
    if (!part) {
      continue;
    }
    if (part.startsWith('$') && part.endsWith('$')) {
      result.push({type: 'latex', id: latexId++, content: part.slice(1, -1)});
    } else {
      result.push({type: 'text', content: part});
    }
  }
  return result;
}

export interface InlineTeXProps {
  /** Text content with $...$ markers for inline LaTeX formulas. */
  content: string;
  /** Font size passed to each formula renderer. Defaults to 16. */
  fontSize?: number;
  /** Default formula color. Explicit LaTeX colors still take precedence. */
  color?: ColorValue;
  /** Style applied to plain-text segments. */
  textStyle?: StyleProp<TextStyle>;
}

/**
 * Renders a mixed string of plain text and `$...$` LaTeX formulas inline.
 *
 * Alignment strategy:
 *   Text segments and formula views are siblings in a flex row.
 *   `alignItems: 'center'` on the row centers all children on the cross axis
 *   automatically — no manual offset calculation required.
 *
 * Sizing strategy:
 *   Each formula is first rendered off-screen (absolute, opacity 0) to
 *   measure its intrinsic size via `onContentSizeChange`. Once all formulas
 *   are measured the row is displayed with each formula at its exact size.
 */
export function InlineTeX({
  content,
  fontSize = 16,
  color,
  textStyle,
}: InlineTeXProps): React.JSX.Element {
  const segments = useMemo(() => parseInlineTeX(content), [content]);
  const [sizes, setSizes] = useState<Record<number, {width: number; height: number}>>({});

  const latexSegs = segments.filter(s => s.type === 'latex') as Extract<
    Segment,
    {type: 'latex'}
  >[];
  const unmeasured = latexSegs.filter(s => sizes[s.id] === undefined);
  const allMeasured = unmeasured.length === 0;

  return (
    <View>
      {/* Off-screen measurement pass */}
      {unmeasured.map(s => (
        <RaTeXView
          key={`m${s.id}`}
          latex={s.content}
          fontSize={fontSize}
          displayMode={false}
          color={color}
          style={styles.measureView}
          onContentSizeChange={e => {
            const {width, height} = e.nativeEvent;
            setSizes(prev => ({...prev, [s.id]: {width, height}}));
          }}
        />
      ))}

      {/* Display pass — flex row, alignItems:'center' handles vertical alignment */}
      {allMeasured ? (
        <View style={styles.row}>
          {segments.map((s, i) => {
            if (s.type === 'text') {
              return (
                <Text key={i} style={textStyle}>
                  {s.content}
                </Text>
              );
            }
            const sz = sizes[s.id];
            return (
              <RaTeXView
                key={i}
                latex={s.content}
                fontSize={fontSize}
                displayMode={false}
                color={color}
                style={{width: sz.width, height: sz.height}}
              />
            );
          })}
        </View>
      ) : (
        <View style={{height: Math.round(fontSize * 1.5)}} />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  row: {flexDirection: 'row', alignItems: 'center', flexWrap: 'wrap'},
  measureView: {
    position: 'absolute',
    opacity: 0,
    top: 0,
    left: 0,
    width: 1000,
    height: 400,
  },
});
