// Main.kt — RaTeX JVM demo: Swing window rendering LaTeX formulas via AWT Graphics2D

package io.ratex.demo

import io.ratex.RaTeXEngine
import io.ratex.RaTeXFontLoader
import io.ratex.RaTeXRenderer
import java.awt.BorderLayout
import java.awt.Color
import java.awt.Dimension
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.RenderingHints
import java.io.File
import javax.imageio.ImageIO
import javax.swing.BorderFactory
import javax.swing.BoxLayout
import javax.swing.JFrame
import javax.swing.JLabel
import javax.swing.JPanel
import javax.swing.JScrollPane
import javax.swing.SwingUtilities
import javax.swing.UIManager

/** Demo formulas: (label, LaTeX). */
private val formulas = listOf(
    "Quadratic formula"        to """\frac{-b \pm \sqrt{b^2-4ac}}{2a}""",
    "Euler's identity"         to """e^{i\pi} + 1 = 0""",
    "Gaussian integral"        to """\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}""",
    "Basel problem"            to """\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}""",
    "Matrix"                   to """\begin{pmatrix}a & b \\ c & d\end{pmatrix}""",
    "Maxwell – Ampère's law"   to """\nabla \times \mathbf{B} = \mu_0\left(\mathbf{J} + \varepsilon_0\frac{\partial \mathbf{E}}{\partial t}\right)""",
    "Binomial theorem"         to """(x+y)^n = \sum_{k=0}^{n} \binom{n}{k} x^k y^{n-k}""",
    "Fourier transform"        to """\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx""",
    "Schrödinger equation"     to """i\hbar\frac{\partial}{\partial t}\Psi = \hat{H}\Psi""",
    "Einstein field equations"  to """G_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4}T_{\mu\nu}""",
    "Gamma function"           to """\Gamma(z) = \int_0^{\infty} t^{z-1}e^{-t}\,dt""",
    "Residue theorem"          to """\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)""",
    "Riemann zeta function"    to """\zeta(s) = \sum_{n=1}^{\infty}\frac{1}{n^s} = \prod_p \frac{1}{1-p^{-s}}""",
    "Stokes' theorem"          to """\oint_{\partial\Sigma}\mathbf{F}\cdot d\mathbf{r} = \iint_{\Sigma}(\nabla\times\mathbf{F})\cdot d\mathbf{S}""",
    "Laplace transform"        to """\mathcal{L}\{f(t)\} = \int_0^{\infty} f(t)\,e^{-st}\,dt""",
    "Taylor series"            to """f(x) = \sum_{n=0}^{\infty}\frac{f^{(n)}(0)}{n!}x^n""",
)

private const val FONT_SIZE = 32f
private const val PADDING = 12

fun main(args: Array<String>) {
    // Locate the fonts directory (relative to repo root)
    val fontsDir = locateFontsDir()
    val loaded = RaTeXFontLoader.loadFromDirectory(fontsDir)
    println("Loaded $loaded KaTeX fonts from: $fontsDir")

    // --png flag: export all formulas as PNG files and exit
    if (args.contains("--png")) {
        exportPng()
        return
    }

    // Launch Swing UI
    SwingUtilities.invokeLater { createAndShowGui() }
}

private fun exportPng() {
    val outDir = File("output")
    outDir.mkdirs()
    for ((label, latex) in formulas) {
        try {
            val dl = RaTeXEngine.parseBlocking(latex)
            val renderer = RaTeXRenderer(dl, FONT_SIZE) { RaTeXFontLoader.getFont(it) }
            val image = renderer.renderToImage(padding = PADDING)
            val fileName = label.replace(Regex("[^a-zA-Z0-9]"), "_") + ".png"
            ImageIO.write(image, "PNG", File(outDir, fileName))
            println("  [OK] $label -> output/$fileName")
        } catch (e: Exception) {
            println("  [ERR] $label: ${e.message}")
        }
    }
    println("Done. ${formulas.size} formulas exported to output/")
}

private fun createAndShowGui() {
    try {
        UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName())
    } catch (_: Exception) {}

    val frame = JFrame("RaTeX JVM Demo").apply {
        defaultCloseOperation = JFrame.EXIT_ON_CLOSE
        preferredSize = Dimension(900, 700)
    }

    val contentPanel = JPanel().apply {
        layout = BoxLayout(this, BoxLayout.Y_AXIS)
        background = Color.WHITE
        border = BorderFactory.createEmptyBorder(16, 24, 16, 24)
    }

    for ((label, latex) in formulas) {
        val formulaPanel = FormulaPanel(label, latex, FONT_SIZE)
        contentPanel.add(formulaPanel)
    }

    val scrollPane = JScrollPane(contentPanel).apply {
        verticalScrollBar.unitIncrement = 16
    }

    frame.contentPane.add(scrollPane, BorderLayout.CENTER)
    frame.pack()
    frame.setLocationRelativeTo(null)
    frame.isVisible = true
}

/** A panel that renders one labeled formula. */
private class FormulaPanel(
    label: String,
    latex: String,
    fontSize: Float,
) : JPanel() {

    private val renderer: RaTeXRenderer?
    private val errorMsg: String?

    init {
        layout = BorderLayout(0, 4)
        background = Color.WHITE
        border = BorderFactory.createCompoundBorder(
            BorderFactory.createMatteBorder(0, 0, 1, 0, Color(0xE0, 0xE0, 0xE0)),
            BorderFactory.createEmptyBorder(8, 0, 8, 0),
        )
        isOpaque = true

        val titleLabel = JLabel(label).apply {
            font = font.deriveFont(12f)
            foreground = Color.GRAY
        }
        add(titleLabel, BorderLayout.NORTH)

        var r: RaTeXRenderer? = null
        var err: String? = null
        try {
            val dl = RaTeXEngine.parseBlocking(latex)
            r = RaTeXRenderer(dl, fontSize) { RaTeXFontLoader.getFont(it) }
        } catch (e: Exception) {
            err = e.message ?: "unknown error"
        }
        renderer = r
        errorMsg = err

        if (renderer != null) {
            val canvas = FormulaCanvas(renderer)
            add(canvas, BorderLayout.CENTER)
        } else {
            val errLabel = JLabel("Error: $errorMsg").apply {
                foreground = Color.RED
            }
            add(errLabel, BorderLayout.CENTER)
        }
    }
}

/** A lightweight panel that draws one formula via RaTeXRenderer. */
private class FormulaCanvas(private val renderer: RaTeXRenderer) : JPanel() {
    init {
        isOpaque = false
        preferredSize = Dimension(
            (renderer.widthPx + PADDING * 2).toInt().coerceAtLeast(1),
            (renderer.totalHeightPx + PADDING * 2).toInt().coerceAtLeast(1),
        )
    }

    override fun paintComponent(g: Graphics) {
        super.paintComponent(g)
        val g2 = g as Graphics2D
        g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
        g2.translate(PADDING, PADDING)
        renderer.draw(g2)
    }
}

/** Walk up from CWD to find the fonts/ directory in the repo root. */
private fun locateFontsDir(): File {
    // Try relative to working directory (expected: demo/jvm or repo root)
    val candidates = listOf(
        File("fonts"),
        File("../../fonts"),
        File("../../../fonts"),
    )
    for (candidate in candidates) {
        if (candidate.isDirectory && File(candidate, "KaTeX_Main-Regular.ttf").exists()) {
            return candidate.canonicalFile
        }
    }
    error("Cannot find KaTeX fonts directory. Run from the repo root or demo/jvm directory.")
}
