import React, {useState} from 'react';
import {
  SafeAreaView,
  ScrollView,
  StatusBar,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
  useColorScheme,
} from 'react-native';
import {InlineTeX, RaTeXView} from 'ratex-react-native';

const SHOWCASE_INLINE_ROWS = [
  String.raw`Einstein showed that mass and energy are $E = mc^2$, where $c$ is the speed of light.`,
  String.raw`A circle of radius $r$ has area $S = \pi r^2$ and circumference $C = 2\pi r$.`,
  String.raw`The golden ratio $\varphi = \frac{1+\sqrt{5}}{2}$ satisfies $\varphi^2 = \varphi + 1$.`,
  String.raw`If $A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}$, then $\det A = ad - bc$.`,
  String.raw`中文：勾股定理是 $\text{勾股定理：} a^2+b^2=c^2$。`,
];

const SHOWCASE_BLOCKS = [
  {
    label: 'Fourier transform',
    latex: String.raw`\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx`,
  },
  {
    label: '3D rotation matrix',
    latex: String.raw`R_z(\theta)=\begin{pmatrix}\cos\theta&-\sin\theta&0\\\sin\theta&\cos\theta&0\\0&0&1\end{pmatrix}`,
  },
  {
    label: 'Schrödinger equation',
    latex: String.raw`i\hbar\frac{\partial}{\partial t}\Psi = \left[-\frac{\hbar^2}{2m}\nabla^2 + V\right]\Psi`,
  },
  {
    label: String.raw`Residue theorem · \operatorname`,
    latex: String.raw`\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)`,
  },
];

const FORMULAS = [
  {name: 'Quadratic formula',    latex: String.raw`\frac{-b \pm \sqrt{b^2-4ac}}{2a}`},
  {name: "Euler's identity",     latex: String.raw`e^{i\pi} + 1 = 0`},
  {name: 'Gaussian integral',    latex: String.raw`\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}`},
  {name: 'Basel problem',        latex: String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}`},
  {name: 'Matrix',               latex: String.raw`\begin{pmatrix}a & b \\ c & d\end{pmatrix}`},
  {name: 'Maxwell',              latex: String.raw`\nabla \times \mathbf{B} = \mu_0 \mathbf{J}`},
  {name: 'CJK · 勾股定理',       latex: String.raw`\text{勾股定理：} a^2+b^2=c^2`},
  {
    name: 'CJK · mhchem + 二氧化碳',
    latex: String.raw`\ce{CO2 + C -> 2 CO} \quad \text{二氧化碳}`,
  },
  {name: 'Emoji · 笑脸',         latex: String.raw`\text{😊} \quad E=mc^2`},
];

const INLINE_EXAMPLES = [
  {
    name: 'Single-line — energy',
    content: String.raw`Mass–energy equivalence: $E = mc^2$, the central result of special relativity.`,
  },
  {
    name: "Single-line — Pythagorean theorem",
    content: String.raw`For a right triangle, the sides satisfy $a^2 + b^2 = c^2$, where c is the hypotenuse.`,
  },
  {
    name: 'Multi-line — integral',
    content: String.raw`The normal distribution is normalised: $\int_{-\infty}^{+\infty} \frac{1}{\sqrt{2\pi}\,\sigma} e^{-\frac{(x-\mu)^2}{2\sigma^2}} dx = 1$, where μ is the mean and σ the standard deviation.`,
  },
  {
    name: 'Multi-line — determinant',
    content: String.raw`The determinant of a 2×2 matrix is $\det\begin{pmatrix}a & b \\ c & d\end{pmatrix} = ad - bc$; the matrix is invertible when this is non-zero.`,
  },
];

export default function App() {
  const isDark = useColorScheme() === 'dark';
  const [custom, setCustom] = useState(String.raw`\frac{1}{\sqrt{2\pi}}`);
  const [fontSize, setFontSize] = useState(28);
  const [error, setError] = useState('');

  return (
    <SafeAreaView style={[styles.root, isDark && styles.dark]}>
      <StatusBar barStyle={isDark ? 'light-content' : 'dark-content'} />
      <ScrollView contentContainerStyle={styles.scroll}>
        <Text style={[styles.title, isDark && styles.textLight]}>RaTeX Demo</Text>

        {/* Showcase — first visible screen */}
        <Text style={[styles.sectionTitle, isDark && styles.textLight]}>
          RaTeX · Native Cross-Platform Math
        </Text>
        <View style={styles.card}>
          <Text style={[styles.label, isDark && styles.textLight]}>
            Inline layout · baseline alignment
          </Text>
          {SHOWCASE_INLINE_ROWS.map((row, i) => (
            <InlineTeX
              key={i}
              content={row}
              fontSize={16}
              textStyle={[styles.inlineText, isDark && styles.textLight]}
            />
          ))}
        </View>
        {SHOWCASE_BLOCKS.map(({label, latex}) => (
          <View key={label} style={styles.card}>
            <Text style={[styles.label, isDark && styles.textLight]}>{label}</Text>
            <RaTeXView latex={latex} fontSize={22} style={styles.formula} />
          </View>
        ))}

        {/* Inline formula examples */}
        <Text style={[styles.sectionTitle, isDark && styles.textLight]}>Inline Layout</Text>
        {INLINE_EXAMPLES.map(({name, content}) => (
          <View key={name} style={styles.card}>
            <Text style={[styles.label, isDark && styles.textLight]}>{name}</Text>
            <InlineTeX
              content={content}
              fontSize={16}
              textStyle={[styles.inlineText, isDark && styles.textLight]}
            />
          </View>
        ))}

        {/* Preset block formulas */}
        <Text style={[styles.sectionTitle, isDark && styles.textLight]}>Formula Examples</Text>
        {FORMULAS.map(({name, latex}) => (
          <View key={name} style={styles.card}>
            <Text style={[styles.label, isDark && styles.textLight]}>{name}</Text>
            <RaTeXView latex={latex} fontSize={24} style={styles.formula} />
          </View>
        ))}

        {/* Custom input */}
        <Text style={[styles.sectionTitle, isDark && styles.textLight]}>Custom Formula</Text>
        <View style={styles.card}>
          <TextInput
            style={styles.input}
            value={custom}
            onChangeText={v => {
              setCustom(v);
              setError('');
            }}
            placeholder="Enter LaTeX…"
            autoCapitalize="none"
          />
          <View style={styles.sizeRow}>
            <TouchableOpacity onPress={() => setFontSize(f => Math.max(14, f - 2))}>
              <Text style={styles.btn}>−</Text>
            </TouchableOpacity>
            <Text style={styles.sizeLabel}>{fontSize}px</Text>
            <TouchableOpacity onPress={() => setFontSize(f => Math.min(48, f + 2))}>
              <Text style={styles.btn}>＋</Text>
            </TouchableOpacity>
          </View>
          {error ? <Text style={styles.err}>{error}</Text> : null}
          <RaTeXView
            latex={custom}
            fontSize={fontSize}
            style={styles.formula}
            onError={e => setError(e.nativeEvent.error)}
          />
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  root: {flex: 1, backgroundColor: '#fff'},
  dark: {backgroundColor: '#111'},
  scroll: {padding: 16, gap: 12},
  title: {fontSize: 22, fontWeight: '700', marginBottom: 8, color: '#111'},
  textLight: {color: '#eee'},
  card: {backgroundColor: '#f5f5f5', borderRadius: 12, padding: 12},
  input: {
    borderWidth: 1,
    borderColor: '#ccc',
    borderRadius: 8,
    padding: 8,
    fontFamily: 'monospace',
  },
  sizeRow: {flexDirection: 'row', alignItems: 'center', gap: 12, marginTop: 8},
  btn: {fontSize: 20, fontWeight: '600', paddingHorizontal: 10},
  sizeLabel: {fontSize: 14, color: '#555'},
  err: {color: 'red', marginTop: 4, fontSize: 12},
  formula: {marginTop: 8, width: '100%'},
  label: {fontSize: 13, color: '#555', marginBottom: 4},
  sectionTitle: {fontSize: 16, fontWeight: '600', color: '#333', marginTop: 4},
  inlineText: {fontSize: 14, color: '#333'},
});
