// RaTeX Flutter Demo
//
// Demonstrates native LaTeX rendering via RaTeXWidget / RaTeXEngine.
//
// Font note: glyph outlines are compiled into the Rust static library
// (libratex_ffi) — no font files are bundled in the Flutter app.

import 'package:flutter/material.dart';
import 'package:ratex_flutter/ratex_flutter.dart';

// ---------------------------------------------------------------------------
// Inline math paragraph helper
//
// Parses a string containing $...$ inline LaTeX segments and builds a
// RichText that intermixes plain TextSpans with WidgetSpans for the math.
// ---------------------------------------------------------------------------

Widget buildInlineMath(
  String text, {
  double mathFontSize = 18,
  TextStyle? textStyle,
}) {
  final style = textStyle ??
      const TextStyle(fontSize: 16, height: 1.8, color: Colors.black87);

  final parts = text.split('\$');
  final spans = <InlineSpan>[];

  for (int i = 0; i < parts.length; i++) {
    if (parts[i].isEmpty) continue;
    if (i.isEven) {
      spans.add(TextSpan(text: parts[i], style: style));
    } else {
      spans.add(WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        baseline: TextBaseline.alphabetic,
        child: RaTeXWidget(
          latex: parts[i],
          fontSize: mathFontSize,
          displayMode: false,
          onError: (e) => debugPrint('RaTeX inline error: $e'),
          loading: const SizedBox.shrink(),
        ),
      ));
    }
  }

  return RichText(text: TextSpan(children: spans));
}

void main() {
  runApp(const RaTeXDemoApp());
}

class RaTeXDemoApp extends StatelessWidget {
  const RaTeXDemoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'RaTeX Demo',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: const DemoPage(),
    );
  }
}

// ---------------------------------------------------------------------------
// Demo page
// ---------------------------------------------------------------------------

class DemoPage extends StatefulWidget {
  const DemoPage({super.key});

  @override
  State<DemoPage> createState() => _DemoPageState();
}

class _DemoPageState extends State<DemoPage> {
  /// Block (`true`) vs inline (`false`) — applies to formula list + custom input.
  bool _displayMode = true;

  static const _formulas = [
    (name: 'Quadratic formula',    latex: r'\frac{-b \pm \sqrt{b^2-4ac}}{2a}'),
    (name: "Euler's identity",     latex: r'e^{i\pi} + 1 = 0'),
    (name: 'Gaussian integral',    latex: r'\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}'),
    (name: 'Basel problem',        latex: r'\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}'),
    (name: 'Matrix',               latex: r'\begin{pmatrix}a & b \\ c & d\end{pmatrix}'),
    (name: 'Maxwell',              latex: r'\nabla \times \mathbf{B} = \mu_0 \mathbf{J}'),
    (name: 'Binomial theorem',     latex: r'(x+y)^n = \sum_{k=0}^n \binom{n}{k} x^k y^{n-k}'),
    (name: 'Middle delimiter',     latex: r'\left( \frac{a}{b} \middle| \frac{c}{d} \right)'),
    (name: 'FTC',                  latex: r'\frac{d}{dx}\left[\int_a^x f(t)\,dt\right] = f(x)'),
    (name: "Stokes' theorem",      latex: r'\oint_{\partial\Sigma} \mathbf{F}\cdot d\mathbf{r} = \iint_\Sigma (\nabla\times\mathbf{F})\cdot d\mathbf{S}'),
  ];

  final _controller = TextEditingController(
    text: r'\frac{d}{dx}\left[\int_a^x f(t)\,dt\right] = f(x)',
  );
  double _fontSize = 24;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('RaTeX Demo'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // ── Showcase — first visible screen ─────────────────────────────
          _SectionHeader('RaTeX · Native Cross-Platform Math'),
          const _ShowcaseCard(),
          const SizedBox(height: 16),
          Card(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      'Preset & custom formulas use:',
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                  ),
                  SegmentedButton<bool>(
                    segments: const [
                      ButtonSegment<bool>(
                        value: true,
                        label: Text('Block'),
                        tooltip: r'Display mode ($$...$$)',
                      ),
                      ButtonSegment<bool>(
                        value: false,
                        label: Text('Inline'),
                        tooltip: r'Inline mode ($...$)',
                      ),
                    ],
                    selected: {_displayMode},
                    onSelectionChanged: (Set<bool> next) {
                      setState(() => _displayMode = next.first);
                    },
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),

          // ── Inline math ──────────────────────────────────────────────────
          _SectionHeader('Inline Layout'),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _label(context, 'Single-line'),
                  const SizedBox(height: 6),
                  buildInlineMath(
                    r"Euler's identity $e^{i\pi}+1=0$ is widely regarded as the most beautiful equation in mathematics.",
                  ),
                  const SizedBox(height: 16),
                  _label(context, 'Multi-line — physics'),
                  const SizedBox(height: 6),
                  buildInlineMath(
                    r'Mass–energy equivalence gives $E = mc^2$, where the speed of light is $c \approx 3\times10^8\ \text{m/s}$.'
                    '\n'
                    r"Momentum $p = mv$ obeys Newton's second law $F = \frac{dp}{dt}$.",
                  ),
                  const SizedBox(height: 16),
                  _label(context, 'Multi-line — mathematics'),
                  const SizedBox(height: 6),
                  buildInlineMath(
                    r'The roots of $ax^2+bx+c=0$ are $x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}$.'
                    '\n'
                    r'The harmonic series $\sum_{n=1}^{\infty}\frac{1}{n}$ diverges, '
                    r'but the alternating series $\sum_{n=1}^{\infty}\frac{(-1)^{n+1}}{n}=\ln 2$ converges.',
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),

          // ── Preset formulas ──────────────────────────────────────────────
          _SectionHeader('Formula Examples'),
          ..._formulas.map((f) => Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: Card(
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(16, 12, 16, 16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _label(context, f.name),
                        const SizedBox(height: 8),
                        _FormulaCard(
                          latex: f.latex,
                          fontSize: 22,
                          displayMode: _displayMode,
                        ),
                      ],
                    ),
                  ),
                ),
              )),

          // ── Custom input ─────────────────────────────────────────────────
          _SectionHeader('Custom Formula'),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  TextField(
                    controller: _controller,
                    decoration: const InputDecoration(
                      labelText: 'Enter LaTeX…',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    style: const TextStyle(fontFamily: 'monospace'),
                    autocorrect: false,
                    onChanged: (_) => setState(() {}),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Text('Size: ${_fontSize.toInt()}px',
                          style: Theme.of(context).textTheme.bodySmall),
                      Expanded(
                        child: Slider(
                          value: _fontSize,
                          min: 14,
                          max: 48,
                          divisions: 17,
                          label: '${_fontSize.toInt()}',
                          onChanged: (v) => setState(() => _fontSize = v),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  _FormulaCard(
                    latex: _controller.text,
                    fontSize: _fontSize,
                    displayMode: _displayMode,
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _label(BuildContext context, String text) => Text(
        text,
        style: Theme.of(context)
            .textTheme
            .labelSmall
            ?.copyWith(color: Colors.grey),
      );
}

// ---------------------------------------------------------------------------
// Showcase card — first-screen highlight
// ---------------------------------------------------------------------------

class _ShowcaseCard extends StatelessWidget {
  const _ShowcaseCard();

  static const _blockItems = [
    (
      label: 'Fourier transform',
      latex: r'\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx',
    ),
    (
      label: '3D rotation matrix',
      latex: r'R_z(\theta)=\begin{pmatrix}\cos\theta&-\sin\theta&0\\\sin\theta&\cos\theta&0\\0&0&1\end{pmatrix}',
    ),
    (
      label: 'Schrödinger equation',
      latex: r'i\hbar\frac{\partial}{\partial t}\Psi = \left[-\frac{\hbar^2}{2m}\nabla^2 + V\right]\Psi',
    ),
    (
      label: r'Residue theorem · \operatorname',
      latex: r'\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)',
    ),
  ];

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Inline layout — multiple rows showing text-before + formula + text-after
            Text('Inline layout · baseline alignment',
                style: Theme.of(context)
                    .textTheme
                    .labelSmall
                    ?.copyWith(color: Colors.grey)),
            const SizedBox(height: 6),
            buildInlineMath(
              r'Einstein showed that mass and energy are $E = mc^2$, where $c$ is the speed of light.',
              mathFontSize: 19,
            ),
            const SizedBox(height: 6),
            buildInlineMath(
              r'A circle of radius $r$ has area $S = \pi r^2$ and circumference $C = 2\pi r$.',
              mathFontSize: 19,
            ),
            const SizedBox(height: 6),
            buildInlineMath(
              r'The golden ratio $\varphi = \frac{1+\sqrt{5}}{2}$ satisfies $\varphi^2 = \varphi + 1$.',
              mathFontSize: 19,
            ),
            const SizedBox(height: 6),
            buildInlineMath(
              r'If $A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}$, then $\det A = ad - bc$.',
              mathFontSize: 19,
            ),
            // Block formulas
            for (final item in _blockItems) ...[
              const Divider(height: 24),
              Text(item.label,
                  style: Theme.of(context)
                      .textTheme
                      .labelSmall
                      ?.copyWith(color: Colors.grey)),
              const SizedBox(height: 8),
              Center(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: RaTeXWidget(
                    latex: item.latex,
                    fontSize: 20,
                    onError: (e) => debugPrint('RaTeX showcase error: $e'),
                    loading: const SizedBox.shrink(),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader(this.title);

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: Text(title,
            style: Theme.of(context)
                .textTheme
                .titleSmall
                ?.copyWith(color: Theme.of(context).colorScheme.primary)),
      );
}

class _FormulaCard extends StatelessWidget {
  final String latex;
  final double fontSize;
  final bool displayMode;

  const _FormulaCard({
    required this.latex,
    required this.fontSize,
    this.displayMode = true,
  });

  @override
  Widget build(BuildContext context) {
    if (latex.trim().isEmpty) return const SizedBox.shrink();
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: RaTeXWidget(
        latex: latex,
        fontSize: fontSize,
        displayMode: displayMode,
        onError: (e) => debugPrint('RaTeX error: $e'),
        loading: const SizedBox.shrink(),
      ),
    );
  }
}
