import SwiftUI

struct ContentView: View {

    let formulas: [(name: String, latex: String)] = [
        ("Quadratic formula",    #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#),
        ("Euler's identity",     #"e^{i\pi} + 1 = 0"#),
        ("Gaussian integral",    #"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}"#),
        ("Basel problem",        #"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}"#),
        ("Matrix",               #"\begin{pmatrix}a & b \\ c & d\end{pmatrix}"#),
        ("Maxwell",              #"\nabla \times \mathbf{B} = \mu_0 \mathbf{J}"#),
        ("Binomial theorem",     #"(x+y)^n = \sum_{k=0}^n \binom{n}{k} x^k y^{n-k}"#),
        ("Middle delimiter",     #"\left( \frac{a}{b} \middle| \frac{c}{d} \right)"#),
        ("Schrödinger equation", #"i\hbar\frac{\partial}{\partial t}\Psi(\mathbf{r},t) = \left[-\frac{\hbar^2}{2m}\nabla^2 + V(\mathbf{r},t)\right]\Psi(\mathbf{r},t)"#),
        ("Taylor series",        #"f(x) = \sum_{n=0}^{\infty}\frac{f^{(n)}(0)}{n!}x^n = f(0) + f'(0)x + \frac{f''(0)}{2!}x^2 + \frac{f'''(0)}{3!}x^3 + \cdots"#),
        ("酸碱中和",             #"\text{酸} + \text{碱} \rightarrow 盐 + 水"#),
    ]

    @State private var customLatex: String = #"\frac{d}{dx}\left[\int_a^x f(t)\,dt\right] = f(x)"#
    @State private var fontSize: Double = 24

    var body: some View {
        NavigationStack {
            List {
                // Showcase — first visible screen
                Section("RaTeX · Native Cross-Platform Math") {
                    ShowcaseView()
                }

                // Inline math mixed with text
                Section("Inline Layout") {
                    InlineExamplesView()
                }

                // Block formulas inside prose paragraphs
                Section("Block Layout") {
                    BlockExamplesView()
                }

                // Step-by-step derivation
                Section("Derivation") {
                    DerivationView()
                }

                // More preset formulas
                Section("More Formulas") {
                    ForEach(formulas, id: \.name) { item in
                        VStack(alignment: .leading, spacing: 8) {
                            Text(item.name)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            RaTeXFormulaCell(latex: item.latex, fontSize: 22)
                        }
                        .padding(.vertical, 4)
                    }
                }

                // Custom input
                Section("Custom Formula") {
                    VStack(alignment: .leading, spacing: 12) {
                        TextField("Enter LaTeX…", text: $customLatex)
                            .font(.system(.body, design: .monospaced))
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()

                        HStack {
                            Text("Size: \(Int(fontSize))pt")
                                .font(.caption)
                            Slider(value: $fontSize, in: 14...48, step: 2)
                        }

                        RaTeXFormulaCell(latex: customLatex, fontSize: CGFloat(fontSize))
                    }
                    .padding(.vertical, 8)
                }
            }
            .navigationTitle("RaTeX Demo")
        }
    }
}

// MARK: - Showcase

/// First-screen highlight: inline baseline alignment, complex nesting,
/// 3×3 matrix, and custom operators — all rendered natively in Rust.
struct ShowcaseView: View {
    private let fs: CGFloat = 19

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {

            // 1. Inline layout — text wrapping around formulas with baseline alignment
            VStack(alignment: .leading, spacing: 10) {
                badge("Inline layout · baseline alignment")

                // 1a: single short formula mid-sentence
                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("Einstein showed that mass and energy are")
                    RaTeXFormula(latex: #"E = mc^2"#, fontSize: fs, onError: { _ in })
                    Text(", where")
                    RaTeXFormula(latex: #"c"#, fontSize: fs, onError: { _ in })
                    Text("is the speed of light.")
                }

                // 1b: two formulas in one sentence
                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("A circle of radius")
                    RaTeXFormula(latex: #"r"#, fontSize: fs, onError: { _ in })
                    Text("has area")
                    RaTeXFormula(latex: #"S = \pi r^2"#, fontSize: fs, onError: { _ in })
                    Text("and circumference")
                    RaTeXFormula(latex: #"C = 2\pi r"#, fontSize: fs, onError: { _ in })
                    Text(".")
                }

                // 1c: fraction inline between text
                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("The golden ratio")
                    RaTeXFormula(latex: #"\varphi = \frac{1+\sqrt{5}}{2}"#, fontSize: fs, onError: { _ in })
                    Text("satisfies")
                    RaTeXFormula(latex: #"\varphi^2 = \varphi + 1"#, fontSize: fs, onError: { _ in })
                    Text(".")
                }

                // 1d: matrix inline between text
                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("If")
                    RaTeXFormula(latex: #"A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}"#, fontSize: fs, onError: { _ in })
                    Text(", then")
                    RaTeXFormula(latex: #"\det A = ad - bc"#, fontSize: fs, onError: { _ in })
                    Text(".")
                }
            }

            Divider()

            // 2. Fourier transform — multi-level exponent nesting
            VStack(alignment: .leading, spacing: 6) {
                badge("Fourier transform")
                ScrollableFormula(
                    latex: #"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx"#,
                    fontSize: 22, onError: { _ in }
                )
                .frame(maxWidth: .infinity, alignment: .center)
            }

            Divider()

            // 3. 3×3 rotation matrix
            VStack(alignment: .leading, spacing: 6) {
                badge("3D rotation matrix")
                ScrollableFormula(
                    latex: #"R_z(\theta)=\begin{pmatrix}\cos\theta&-\sin\theta&0\\\sin\theta&\cos\theta&0\\0&0&1\end{pmatrix}"#,
                    fontSize: 21, onError: { _ in }
                )
                .frame(maxWidth: .infinity, alignment: .center)
            }

            Divider()

            // 4. Time-dependent Schrödinger equation
            VStack(alignment: .leading, spacing: 6) {
                badge("Schrödinger equation")
                ScrollableFormula(
                    latex: #"i\hbar\frac{\partial}{\partial t}\Psi = \left[-\frac{\hbar^2}{2m}\nabla^2 + V\right]\Psi"#,
                    fontSize: 21, onError: { _ in }
                )
                .frame(maxWidth: .infinity, alignment: .center)
            }

            Divider()

            // 5. Residue theorem — \operatorname
            VStack(alignment: .leading, spacing: 6) {
                badge("Residue theorem · \\operatorname")
                ScrollableFormula(
                    latex: #"\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)"#,
                    fontSize: 21, onError: { _ in }
                )
                .frame(maxWidth: .infinity, alignment: .center)
            }
        }
        .padding(.vertical, 6)
    }

    private func badge(_ text: String) -> some View {
        Text(text)
            .font(.caption.bold())
            .foregroundStyle(.secondary)
    }
}

// MARK: - FlowLayout

/// Wrapping layout that places children left-to-right and wraps to the next
/// line when the available width is exceeded. Within each line, items are
/// baseline-aligned using RaTeXFormulaAscentKey for math views and
/// SwiftUI's firstTextBaseline for plain Text views.
struct FlowLayout: Layout {
    var horizontalSpacing: CGFloat = 4
    var lineSpacing: CGFloat = 6

    typealias Cache = [[(index: Int, size: CGSize)]]
    func makeCache(subviews: Subviews) -> Cache { [] }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) -> CGSize {
        let maxWidth = proposal.width ?? .infinity
        cache = lines(for: subviews, maxWidth: maxWidth)

        var h: CGFloat = 0
        for (i, line) in cache.enumerated() {
            h += line.map(\.size.height).max() ?? 0
            if i < cache.count - 1 { h += lineSpacing }
        }
        return CGSize(width: maxWidth, height: h)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) {
        if cache.isEmpty {
            cache = lines(for: subviews, maxWidth: bounds.width)
        }

        var y = bounds.minY
        for line in cache {
            let baselines: [CGFloat] = line.map { item in
                let customAscent = subviews[item.index][RaTeXFormulaAscentKey.self]
                if customAscent > 0 { return customAscent }
                let d = subviews[item.index].dimensions(in: ProposedViewSize(item.size))
                let native = d[.firstTextBaseline]
                return native > 0 ? native : item.size.height / 2
            }

            let maxBaseline = baselines.max() ?? 0
            let lineHeight  = line.map(\.size.height).max() ?? 0

            var x = bounds.minX
            for (i, item) in line.enumerated() {
                subviews[item.index].place(
                    at: CGPoint(x: x, y: y + (maxBaseline - baselines[i])),
                    proposal: ProposedViewSize(item.size)
                )
                x += item.size.width + horizontalSpacing
            }
            y += lineHeight + lineSpacing
        }
    }

    private func lines(for subviews: Subviews, maxWidth: CGFloat) -> Cache {
        var result: Cache = []
        var currentLine: [(index: Int, size: CGSize)] = []
        var currentX: CGFloat = 0

        for (i, subview) in subviews.enumerated() {
            let size = subview.sizeThatFits(.unspecified)
            let needed = currentLine.isEmpty ? size.width : size.width + horizontalSpacing

            if currentX + needed > maxWidth, !currentLine.isEmpty {
                result.append(currentLine)
                currentLine = [(i, size)]
                currentX = size.width
            } else {
                if !currentLine.isEmpty { currentX += horizontalSpacing }
                currentLine.append((i, size))
                currentX += size.width
            }
        }
        if !currentLine.isEmpty { result.append(currentLine) }
        return result
    }
}

// MARK: - Inline Examples

struct InlineExamplesView: View {
    private let fs: CGFloat = 17

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {

            // Example 1: Newton's second law
            VStack(alignment: .leading, spacing: 6) {
                Label("Newton's second law", systemImage: "atom")
                    .font(.caption.bold()).foregroundStyle(.secondary)

                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("The net force")
                    RaTeXFormula(latex: #"F"#,      fontSize: fs, onError: { _ in })
                    Text("on an object equals mass")
                    RaTeXFormula(latex: #"m"#,      fontSize: fs, onError: { _ in })
                    Text("times acceleration")
                    RaTeXFormula(latex: #"a"#,      fontSize: fs, onError: { _ in })
                    Text(", i.e.")
                    RaTeXFormula(latex: #"F = ma"#, fontSize: fs, onError: { _ in })
                    Text("(unit: N = kg·m/s²).")
                }
            }

            Divider()

            // Example 2: Circle geometry
            VStack(alignment: .leading, spacing: 6) {
                Label("Circle geometry", systemImage: "circle")
                    .font(.caption.bold()).foregroundStyle(.secondary)

                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("A circle of radius")
                    RaTeXFormula(latex: #"r"#,                   fontSize: fs, onError: { _ in })
                    Text("has area")
                    RaTeXFormula(latex: #"S = \pi r^2"#,         fontSize: fs, onError: { _ in })
                    Text("and circumference")
                    RaTeXFormula(latex: #"C = 2\pi r"#,          fontSize: fs, onError: { _ in })
                    Text(", where")
                    RaTeXFormula(latex: #"\pi \approx 3.14159"#, fontSize: fs, onError: { _ in })
                    Text(".")
                }
            }

            Divider()

            // Example 3: Discriminant
            VStack(alignment: .leading, spacing: 6) {
                Label("Discriminant", systemImage: "function")
                    .font(.caption.bold()).foregroundStyle(.secondary)

                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("The discriminant")
                    RaTeXFormula(latex: #"\Delta = b^2 - 4ac"#, fontSize: fs, onError: { _ in })
                    Text("determines the number of roots:")
                    RaTeXFormula(latex: #"\Delta > 0"#,          fontSize: fs, onError: { _ in })
                    Text("two distinct roots,")
                    RaTeXFormula(latex: #"\Delta = 0"#,          fontSize: fs, onError: { _ in })
                    Text("one repeated root,")
                    RaTeXFormula(latex: #"\Delta < 0"#,          fontSize: fs, onError: { _ in })
                    Text("no real roots.")
                }
            }

            Divider()

            // Example 4: Conservation of mechanical energy
            VStack(alignment: .leading, spacing: 6) {
                Label("Conservation of energy", systemImage: "bolt.circle")
                    .font(.caption.bold()).foregroundStyle(.secondary)

                FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
                    Text("When only gravity acts, kinetic energy")
                    RaTeXFormula(latex: #"E_k"#,            fontSize: fs, onError: { _ in })
                    Text("plus potential energy")
                    RaTeXFormula(latex: #"E_p"#,            fontSize: fs, onError: { _ in })
                    Text("is conserved:")
                    RaTeXFormula(latex: #"E_k + E_p = C"#, fontSize: fs, onError: { _ in })
                    Text("(C constant).")
                }
            }

            Divider()

            // Example 5: Determinant
            VStack(alignment: .leading, spacing: 6) {
                Label("Determinant", systemImage: "grid")
                    .font(.caption.bold()).foregroundStyle(.secondary)

                FlowLayout(horizontalSpacing: 3, lineSpacing: 8) {
                    Text("If")
                    RaTeXFormula(latex: #"A = \begin{vmatrix} a & b \\ c & d \end{vmatrix}"#, fontSize: fs, onError: { _ in })
                    Text(", then")
                    RaTeXFormula(latex: #"\det(A) = ad - bc"#, fontSize: fs, onError: { _ in })
                    Text(". For a 3×3 determinant")
                    RaTeXFormula(latex: #"B = \begin{vmatrix} a & b & c \\ d & e & f \\ g & h & i \end{vmatrix}"#, fontSize: fs, onError: { _ in })
                    Text("expanded along the first row:")
                    RaTeXFormula(latex: #"\det(B) = a(ei-fh) - b(di-fg) + c(dh-eg)"#, fontSize: fs, onError: { _ in })
                    Text(".")
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Block Examples

struct BlockExamplesView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {

            // Example 1: Mass–energy equivalence
            VStack(alignment: .leading, spacing: 8) {
                Label("Mass–energy equivalence", systemImage: "bolt.fill")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)

                Text("Einstein derived from special relativity that the rest energy of a body with mass m is")
                    .fixedSize(horizontal: false, vertical: true)

                ScrollableFormula(latex: #"E = mc^2"#, fontSize: 28, onError: { _ in })
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 4)

                Text("where c ≈ 3×10⁸ m/s is the speed of light in vacuum. It shows that mass and energy are interconvertible.")
                    .fixedSize(horizontal: false, vertical: true)
                    .foregroundStyle(.secondary)
            }
            .padding(.vertical, 4)

            Divider()

            // Example 2: Maxwell's equations (integral form)
            VStack(alignment: .leading, spacing: 8) {
                Label("Maxwell's equations (integral form)", systemImage: "wave.3.right")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)

                Text("The electromagnetic field is fully described by these four equations:")

                VStack(alignment: .leading, spacing: 10) {
                    ForEach(maxwellEquations, id: \.label) { eq in
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text(eq.label)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 90, alignment: .trailing)
                            ScrollableFormula(latex: eq.latex, fontSize: 18, onError: { _ in })
                        }
                    }
                }
                .padding(.vertical, 4)
            }
            .padding(.vertical, 4)

            Divider()

            // Example 3: Taylor series
            VStack(alignment: .leading, spacing: 8) {
                Label("Taylor series", systemImage: "chart.line.uptrend.xyaxis")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)

                Text("The Taylor series of f(x) around x = a is:")
                    .fixedSize(horizontal: false, vertical: true)

                ScrollableFormula(
                    latex: #"f(x) = \sum_{n=0}^{\infty} \frac{f^{(n)}(a)}{n!}(x-a)^n"#,
                    fontSize: 20,
                    onError: { _ in }
                )
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, 4)

                Text("Common expansions:")

                VStack(alignment: .leading, spacing: 8) {
                    ForEach(taylorExamples, id: \.name) { item in
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text(item.name)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 36, alignment: .trailing)
                            ScrollableFormula(latex: item.latex, fontSize: 17, onError: { _ in })
                        }
                    }
                }
            }
            .padding(.vertical, 4)
        }
        .padding(.vertical, 4)
    }

    private let maxwellEquations: [(label: String, latex: String)] = [
        ("Gauss's law",      #"\oint \mathbf{E} \cdot d\mathbf{A} = \frac{Q}{\varepsilon_0}"#),
        ("Gauss (magn.)",    #"\oint \mathbf{B} \cdot d\mathbf{A} = 0"#),
        ("Faraday's law",    #"\oint \mathbf{E} \cdot d\mathbf{l} = -\frac{d\Phi_B}{dt}"#),
        ("Ampère's law",     #"\oint \mathbf{B} \cdot d\mathbf{l} = \mu_0 I + \mu_0\varepsilon_0 \frac{d\Phi_E}{dt}"#),
    ]

    private let taylorExamples: [(name: String, latex: String)] = [
        ("eˣ",    #"e^x = 1 + x + \frac{x^2}{2!} + \frac{x^3}{3!} + \cdots"#),
        ("sin x", #"\sin x = x - \frac{x^3}{3!} + \frac{x^5}{5!} - \cdots"#),
        ("cos x", #"\cos x = 1 - \frac{x^2}{2!} + \frac{x^4}{4!} - \cdots"#),
    ]
}

// MARK: - Derivation

struct DerivationView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label("Deriving the quadratic formula", systemImage: "graduationcap")
                .font(.caption.bold())
                .foregroundStyle(.secondary)
                .padding(.bottom, 4)

            ForEach(derivationSteps.indices, id: \.self) { i in
                let step = derivationSteps[i]
                VStack(alignment: .leading, spacing: 6) {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text("\(i + 1).")
                            .font(.caption.bold())
                            .foregroundStyle(.secondary)
                            .frame(width: 20, alignment: .trailing)
                        Text(step.description)
                            .font(.subheadline)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    if let latex = step.latex {
                        ScrollableFormula(latex: latex, fontSize: 20, onError: { _ in })
                            .frame(maxWidth: .infinity, alignment: .center)
                            .padding(.vertical, 4)
                            .padding(.leading, 26)
                    }
                }
                .padding(.vertical, 4)

                if i < derivationSteps.count - 1 {
                    Divider().padding(.leading, 26)
                }
            }
        }
        .padding(.vertical, 4)
    }

    private struct Step {
        let description: String
        let latex: String?
    }

    private let derivationSteps: [Step] = [
        Step(description: "Start from the standard form",
             latex: #"ax^2 + bx + c = 0 \quad (a \neq 0)"#),
        Step(description: "Divide both sides by a",
             latex: #"x^2 + \frac{b}{a}x + \frac{c}{a} = 0"#),
        Step(description: "Complete the square",
             latex: #"\left(x + \frac{b}{2a}\right)^2 = \frac{b^2 - 4ac}{4a^2}"#),
        Step(description: "Take the square root of both sides (±)",
             latex: #"x + \frac{b}{2a} = \pm\frac{\sqrt{b^2 - 4ac}}{2a}"#),
        Step(description: "Isolate x to obtain the quadratic formula",
             latex: #"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#),
    ]
}

// MARK: - Formula Cell

struct RaTeXFormulaCell: View {
    let latex: String
    let fontSize: CGFloat

    @State private var errorMessage: String?

    var body: some View {
        Group {
            if let err = errorMessage {
                Label(err, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(.red)
            } else {
                ScrollableFormula(
                    latex: latex,
                    fontSize: fontSize,
                    onError: { e in errorMessage = e.localizedDescription }
                )
            }
        }
        .onChange(of: latex) { _ in errorMessage = nil }
    }
}

// MARK: - Scrollable formula (for very long equations)

/// Wrap a single formula in a horizontal scroll view so long expressions can be inspected.
/// This keeps the formula's intrinsic size, and scrolls only when it exceeds the available width.
struct ScrollableFormula: View {
    let latex: String
    let fontSize: CGFloat
    var displayMode: Bool = true
    var onError: ((Error) -> Void)? = nil

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            RaTeXFormula(latex: latex, fontSize: fontSize, displayMode: displayMode, onError: onError)
                .fixedSize(horizontal: true, vertical: true)
                .padding(.vertical, 2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

#Preview {
    ContentView()
}
