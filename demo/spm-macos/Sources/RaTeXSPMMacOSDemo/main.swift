import AppKit
import RaTeX
import SwiftUI

private struct FormulaSample: Identifiable {
    let id = UUID()
    let title: String
    let latex: String
    let displayMode: Bool
    let color: Color
    var showsMixedLayout = false
}

private let samples: [FormulaSample] = [
    FormulaSample(
        title: "Mixed inline layout",
        latex: #"E = mc^2"#,
        displayMode: false,
        color: .teal,
        showsMixedLayout: true
    ),
    FormulaSample(
        title: "Quadratic formula",
        latex: #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#,
        displayMode: true,
        color: .blue
    ),
    FormulaSample(
        title: "Gaussian integral",
        latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#,
        displayMode: true,
        color: .purple
    ),
    FormulaSample(
        title: "Matrix",
        latex: #"\begin{pmatrix}a&b\\c&d\end{pmatrix}"#,
        displayMode: true,
        color: .green
    ),
    FormulaSample(
        title: "Inline identity",
        latex: #"e^{i\pi}+1=0"#,
        displayMode: false,
        color: .orange
    ),
]

@main
struct RaTeXSPMMacOSDemoApp: App {
    init() {
        RaTeXFontLoader.ensureLoaded()
    }

    var body: some Scene {
        WindowGroup("RaTeX SPM macOS Demo") {
            DemoView()
                .frame(minWidth: 720, minHeight: 460)
        }
    }
}

private struct DemoView: View {
    @State private var selectedSampleID: FormulaSample.ID? = samples[0].id
    @State private var parseSummary = "Waiting for render"
    @State private var lastError: String?

    private var selectedSample: FormulaSample {
        samples.first { $0.id == selectedSampleID } ?? samples[0]
    }

    var body: some View {
        NavigationSplitView {
            List(samples, selection: $selectedSampleID) { sample in
                Text(sample.title)
                    .tag(sample.id)
            }
            .navigationTitle("Formulas")
        } detail: {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 8) {
                    Text(selectedSample.title)
                        .font(.title2.weight(.semibold))

                    if selectedSample.showsMixedLayout {
                        Text("Text and RaTeXFormula views sharing one wrapping baseline layout")
                            .foregroundStyle(.secondary)
                    } else {
                        Text(selectedSample.latex)
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                            .foregroundStyle(.secondary)
                    }
                }

                Divider()

                if selectedSample.showsMixedLayout {
                    MixedFormulaShowcase()
                        .ratexColor(.primary)
                } else {
                    RaTeXFormula(
                        latex: selectedSample.latex,
                        fontSize: selectedSample.displayMode ? 34 : 26,
                        displayMode: selectedSample.displayMode,
                        color: selectedSample.color,
                        onError: { error in
                            lastError = error.localizedDescription
                        },
                        onLayout: { ascent, totalHeight in
                            parseSummary = String(
                                format: "Rendered by SPM package: ascent %.1f pt, total height %.1f pt",
                                ascent,
                                totalHeight
                            )
                            lastError = nil
                        }
                    )
                    .ratexColor(.primary)
                    .padding(.vertical, 16)
                }

                if let lastError {
                    Text(lastError)
                        .font(.callout)
                        .foregroundStyle(.red)
                } else {
                    Text(parseSummary)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }
            .padding(28)
            .onAppear(perform: validateEngine)
            .onChange(of: selectedSampleID) { _, _ in
                validateEngine()
            }
        }
    }

    private func validateEngine() {
        do {
            if selectedSample.showsMixedLayout {
                let displayLists = try mixedFormulaInputs.map { input in
                    try RaTeXEngine.shared.parse(
                        input.latex,
                        displayMode: input.displayMode,
                        color: .labelColor
                    )
                }
                let itemCount = displayLists.reduce(0) { $0 + $1.items.count }
                parseSummary = "Parsed mixed layout sample: \(displayLists.count) formulas, \(itemCount) display-list items"
                lastError = nil
                return
            }

            let displayList = try RaTeXEngine.shared.parse(
                selectedSample.latex,
                displayMode: selectedSample.displayMode,
                color: .labelColor
            )
            parseSummary = String(
                format: "Parsed by SPM package: %.2f em x %.2f em, %d items",
                displayList.width,
                displayList.height + displayList.depth,
                displayList.items.count
            )
            lastError = nil
        } catch {
            lastError = error.localizedDescription
        }
    }
}

private let mixedFormulaInputs: [(latex: String, displayMode: Bool)] = [
    (#"E = mc^2"#, false),
    (#"c"#, false),
    (#"r"#, false),
    (#"S = \pi r^2"#, false),
    (#"C = 2\pi r"#, false),
    (#"\varphi = \frac{1+\sqrt{5}}{2}"#, false),
    (#"\varphi^2 = \varphi + 1"#, false),
    (#"A = \begin{pmatrix}a & b \\ c & d\end{pmatrix}"#, false),
    (#"\det A = ad - bc"#, false),
    (#"\int_0^1 x^2\,dx = \frac{1}{3}"#, true),
]

private struct MixedFormulaShowcase: View {
    private let inlineFontSize: CGFloat = 18

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                MixedFormulaSection(title: "Physics") {
                    BaselineFlowLayout(horizontalSpacing: 4, lineSpacing: 7) {
                        Text("Einstein's mass-energy relation is")
                        InlineFormula(#"E = mc^2"#)
                        Text(", where")
                        InlineFormula(#"c"#)
                        Text("is the speed of light.")
                    }
                }

                Divider()

                MixedFormulaSection(title: "Geometry") {
                    BaselineFlowLayout(horizontalSpacing: 4, lineSpacing: 7) {
                        Text("A circle with radius")
                        InlineFormula(#"r"#)
                        Text("has area")
                        InlineFormula(#"S = \pi r^2"#)
                        Text("and circumference")
                        InlineFormula(#"C = 2\pi r"#)
                        Text(".")
                    }
                }

                Divider()

                MixedFormulaSection(title: "Algebra") {
                    BaselineFlowLayout(horizontalSpacing: 4, lineSpacing: 8) {
                        Text("The golden ratio")
                        InlineFormula(#"\varphi = \frac{1+\sqrt{5}}{2}"#)
                        Text("satisfies")
                        InlineFormula(#"\varphi^2 = \varphi + 1"#)
                        Text(". For")
                        InlineFormula(#"A = \begin{pmatrix}a & b \\ c & d\end{pmatrix}"#)
                        Text("we have")
                        InlineFormula(#"\det A = ad - bc"#)
                        Text(".")
                    }
                }

                Divider()

                MixedFormulaSection(title: "Block formula inside prose") {
                    Text("The same SPM package also renders display-style formulas between paragraphs.")

                    RaTeXFormula(
                        latex: #"\int_0^1 x^2\,dx = \frac{1}{3}"#,
                        fontSize: 30,
                        displayMode: true
                    )
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 4)

                    Text("The surrounding SwiftUI text keeps normal wrapping while the formula uses RaTeX's native renderer.")
                }
            }
            .padding(.vertical, 8)
        }
    }

    private func InlineFormula(_ latex: String) -> some View {
        RaTeXFormula(
            latex: latex,
            fontSize: inlineFontSize,
            displayMode: false
        )
    }
}

private struct MixedFormulaSection<Content: View>: View {
    let title: String
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.weight(.semibold))
                .foregroundStyle(.secondary)
            content
        }
    }
}

private struct BaselineFlowLayout: Layout {
    var horizontalSpacing: CGFloat = 4
    var lineSpacing: CGFloat = 6

    typealias Cache = [[(index: Int, size: CGSize)]]

    func makeCache(subviews: Subviews) -> Cache { [] }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) -> CGSize {
        let maxWidth = proposal.width ?? .infinity
        cache = lines(for: subviews, maxWidth: maxWidth)

        let height = cache.enumerated().reduce(CGFloat.zero) { partial, entry in
            let lineHeight = entry.element.map(\.size.height).max() ?? 0
            let spacing = entry.offset < cache.count - 1 ? lineSpacing : 0
            return partial + lineHeight + spacing
        }

        return CGSize(width: maxWidth, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) {
        if cache.isEmpty {
            cache = lines(for: subviews, maxWidth: bounds.width)
        }

        var y = bounds.minY
        for line in cache {
            let baselines = line.map { item in
                let dimensions = subviews[item.index].dimensions(in: ProposedViewSize(item.size))
                let textBaseline = dimensions[.firstTextBaseline]
                return textBaseline > 0 ? textBaseline : item.size.height / 2
            }

            let maxBaseline = baselines.max() ?? 0
            let lineHeight = line.map(\.size.height).max() ?? 0

            var x = bounds.minX
            for (offset, item) in line.enumerated() {
                subviews[item.index].place(
                    at: CGPoint(x: x, y: y + maxBaseline - baselines[offset]),
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
        var currentWidth: CGFloat = 0

        for (index, subview) in subviews.enumerated() {
            let size = subview.sizeThatFits(.unspecified)
            let proposedWidth = currentLine.isEmpty ? size.width : currentWidth + horizontalSpacing + size.width

            if proposedWidth > maxWidth, !currentLine.isEmpty {
                result.append(currentLine)
                currentLine = [(index, size)]
                currentWidth = size.width
            } else {
                currentLine.append((index, size))
                currentWidth = proposedWidth
            }
        }

        if !currentLine.isEmpty {
            result.append(currentLine)
        }

        return result
    }
}
