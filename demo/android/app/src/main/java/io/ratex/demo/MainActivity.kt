package io.ratex.demo

import android.os.Bundle
import android.text.Editable
import android.text.SpannableStringBuilder
import android.text.TextWatcher
import android.util.TypedValue
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import io.ratex.RaTeXException
import io.ratex.RaTeXSpan
import io.ratex.RaTeXView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    // ── Showcase — first visible screen ──────────────────────────────────────

    /** Inline sentences with $...$ math markers — multiple rows for the showcase. */
    private val showcaseInlineRows = listOf(
        "Einstein showed that mass and energy are \$E = mc^2$, where \$c$ is the speed of light.",
        "A circle of radius \$r$ has area \$S = \\pi r^2$ and circumference \$C = 2\\pi r$.",
        "The golden ratio \$\\varphi = \\frac{1+\\sqrt{5}}{2}$ satisfies \$\\varphi^2 = \\varphi + 1$.",
        "If \$A = \\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$, then \$\\det A = ad - bc$.",
        "中文：勾股定理是 \$\text{勾股定理：} a^2+b^2=c^2$。",
    )

    /** Block formulas for the showcase section: (label, LaTeX). */
    private val showcaseBlocks = listOf(
        "Fourier transform" to
            """\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx""",
        "3D rotation matrix" to
            """R_z(\theta)=\begin{pmatrix}\cos\theta&-\sin\theta&0\\\sin\theta&\cos\theta&0\\0&0&1\end{pmatrix}""",
        "Schrödinger equation" to
            """i\hbar\frac{\partial}{\partial t}\Psi = \left[-\frac{\hbar^2}{2m}\nabla^2 + V\right]\Psi""",
        """Residue theorem · \operatorname""" to
            """\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)""",
    )

    // ── Preset block formulas ─────────────────────────────────────────────────
    private val formulas = listOf(
        "Quadratic formula"         to """\frac{-b \pm \sqrt{b^2-4ac}}{2a}""",
        "Euler's identity"          to """e^{i\pi} + 1 = 0""",
        "Gaussian integral"         to """\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}""",
        "Basel problem"             to """\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}""",
        "Matrix"                    to """\begin{pmatrix}a & b \\ c & d\end{pmatrix}""",
        "Maxwell – Ampère's law"    to """\nabla \times \mathbf{B} = \mu_0\left(\mathbf{J} + \varepsilon_0\frac{\partial \mathbf{E}}{\partial t}\right)""",
        "Binomial theorem"          to """(x+y)^n = \sum_{k=0}^{n} \binom{n}{k} x^k y^{n-k}""",
        "Fourier transform"         to """\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\,e^{-2\pi i x \xi}\,dx""",
        "Schrödinger equation"      to """i\hbar\frac{\partial}{\partial t}\Psi = \hat{H}\Psi""",
        "Einstein field equations"  to """G_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4}T_{\mu\nu}""",
        "Gamma function"            to """\Gamma(z) = \int_0^{\infty} t^{z-1}e^{-t}\,dt""",
        "Residue theorem"           to """\oint_C f(z)\,dz = 2\pi i \sum_k \operatorname{Res}(f,z_k)""",
        "Riemann zeta function"     to """\zeta(s) = \sum_{n=1}^{\infty}\frac{1}{n^s} = \prod_p \frac{1}{1-p^{-s}}""",
        "Bessel function"           to """J_n(x) = \frac{1}{\pi}\int_0^{\pi}\cos(n\tau - x\sin\tau)\,d\tau""",
        "Stokes' theorem"           to """\oint_{\partial\Sigma}\mathbf{F}\cdot d\mathbf{r} = \iint_{\Sigma}(\nabla\times\mathbf{F})\cdot d\mathbf{S}""",
        "Laplace transform"         to """\mathcal{L}\{f(t)\} = \int_0^{\infty} f(t)\,e^{-st}\,dt""",
        "CJK · 勾股定理"             to """\text{勾股定理：} a^2+b^2=c^2""",
        "CJK · mhchem + 二氧化碳"   to """\ce{CO2 + C -> 2 CO} \quad \text{二氧化碳}""",
        "Emoji · 笑脸"               to """\text{😊} \quad E=mc^2""",
    )

    // ── Inline mix rows: (prefix, LaTeX, suffix) ──────────────────────────────
    private val inlineFormulas = listOf(
        Triple("Golden ratio φ = ", """\frac{1+\sqrt{5}}{2}""", " ≈ 1.618, found throughout nature and art."),
        Triple("By the Pythagorean theorem ", """a^2 + b^2 = c^2""", ", the hypotenuse of a right triangle is directly obtained."),
        Triple("For x > 0, ", """\frac{d}{dx}\ln x = \frac{1}{x}""", " holds everywhere on the positive real line."),
        Triple("The modulus of a complex number |z| = ", """\sqrt{a^2+b^2}""", ", where z = a + bi."),
        Triple("As n→∞, ", """\left(1+\frac{1}{n}\right)^n""", " converges to the natural constant e ≈ 2.71828."),
        Triple("The integral ", """\int_0^{\pi}\sin x\,dx""", " = 2 is a fundamental definite integral result."),
        Triple("Expanding ", """\sum_{k=0}^{n}\binom{n}{k}x^k y^{n-k}""", " gives the full binomial theorem."),
        Triple("The important limit: ", """\lim_{x \to 0} \frac{\sin x}{x}""", " = 1 is the cornerstone of trigonometric limits."),
        Triple("The substitution t = ", """\frac{x-\mu}{\sigma}""", " converts a general normal integral to standard form."),
        Triple("The determinant det(A) = ", """\begin{vmatrix}a & b \\ c & d\end{vmatrix}""", " = ad − bc is a core quantity in linear algebra."),
        Triple("Stirling's approximation: ", """n! \approx \sqrt{2\pi n}\left(\frac{n}{e}\right)^n""", ", with relative error shrinking as n grows."),
        Triple("Normal density p(x) = ", """\frac{1}{\sigma\sqrt{2\pi}}\,e^{-\frac{(x-\mu)^2}{2\sigma^2}}""", ", where μ is the mean and σ the standard deviation."),
        Triple("Cauchy–Schwarz: ", """\left|\sum_k a_k b_k\right|^2 \le \sum_k a_k^2\cdot\sum_k b_k^2""", ", equality holds iff the sequences are proportional."),
        Triple("Green's theorem ", """\oint_C (P\,dx+Q\,dy) = \iint_D\!\left(\frac{\partial Q}{\partial x}-\frac{\partial P}{\partial y}\right)dA""", " converts a line integral to a double integral."),
    )

    // ── Multiline formulas: (description, LaTeX) ──────────────────────────────
    private val multilineFormulas = listOf(
        "The time-dependent Schrödinger equation governs quantum state evolution, where Ĥ is the Hamiltonian:" to
            """i\hbar\frac{\partial}{\partial t}\Psi(\mathbf{r},t) = \left[-\frac{\hbar^2}{2m}\nabla^2 + V(\mathbf{r},t)\right]\Psi(\mathbf{r},t)""",

        "Maxwell's equations (differential form) — Faraday's induction law and the Ampère–Maxwell law:" to
            """\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}, \quad \nabla \times \mathbf{B} = \mu_0\mathbf{J} + \mu_0\varepsilon_0\frac{\partial \mathbf{E}}{\partial t}""",

        "Einstein's field equations of general relativity — spacetime curvature on the left, energy–momentum tensor on the right:" to
            """R_{\mu\nu} - \frac{1}{2}R\,g_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4}T_{\mu\nu}""",

        "The incompressible Navier–Stokes equation describes viscous fluid motion:" to
            """\rho\!\left(\frac{\partial \mathbf{v}}{\partial t} + \mathbf{v}\cdot\nabla\mathbf{v}\right) = -\nabla p + \mu\nabla^2\mathbf{v} + \mathbf{f}""",

        "Taylor series (Maclaurin form), valid when f is infinitely differentiable at 0:" to
            """f(x) = \sum_{n=0}^{\infty}\frac{f^{(n)}(0)}{n!}x^n = f(0) + f'(0)x + \frac{f''(0)}{2!}x^2 + \cdots""",

        "The Euler–Lagrange equation is the condition for a functional to be stationary:" to
            """\frac{\partial \mathcal{L}}{\partial q} - \frac{d}{dt}\frac{\partial \mathcal{L}}{\partial \dot{q}} = 0""",

        "The Feynman path integral describes the propagator of a quantum system:" to
            """\langle x_f,t_f \mid x_i,t_i \rangle = \int \mathcal{D}[x(t)]\,\exp\!\left(\frac{i}{\hbar}\int_{t_i}^{t_f}\mathcal{L}\,dt\right)""",

        "The Riemann zeta function satisfies the following functional (reflection) equation:" to
            """\zeta(s) = 2^s\pi^{s-1}\sin\!\left(\frac{\pi s}{2}\right)\Gamma(1-s)\,\zeta(1-s)""",
    )

    private val defaultLatex = """\frac{d}{dx}\left[\int_a^x f(t)\,dt\right] = f(x)"""
    private val fontSizeDp = 22f

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        var currentFontSizeDp = fontSizeDp

        val customLatex = findViewById<android.widget.EditText>(R.id.customLatex)
        val fontSizeLabel = findViewById<TextView>(R.id.fontSizeLabel)
        val fontSizeSeekBar = findViewById<SeekBar>(R.id.fontSizeSeekBar)
        val customMathView = findViewById<RaTeXView>(R.id.customMathView)
        val showcaseContainer = findViewById<LinearLayout>(R.id.showcaseContainer)
        val formulaExamplesContainer = findViewById<LinearLayout>(R.id.formulaExamplesContainer)
        val inlineMixContainer = findViewById<LinearLayout>(R.id.inlineMixContainer)
        val multilineMixContainer = findViewById<LinearLayout>(R.id.multilineMixContainer)

        customLatex.setText(defaultLatex)
        updateFontSizeLabel(fontSizeLabel, fontSizeSeekBar.progress)
        customMathView.onError = { e: RaTeXException ->
            Toast.makeText(this, e.message, Toast.LENGTH_SHORT).show()
        }
        customMathView.fontSize = currentFontSizeDp
        customMathView.latex = customLatex.text.toString()

        customLatex.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: Editable?) {
                customMathView.latex = s?.toString() ?: ""
            }
        })

        fontSizeSeekBar.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(seekBar: SeekBar?, progress: Int, fromUser: Boolean) {
                updateFontSizeLabel(fontSizeLabel, progress)
                currentFontSizeDp = progress.toFloat()
                customMathView.fontSize = currentFontSizeDp
                customMathView.latex = customLatex.text.toString()
            }
            override fun onStartTrackingTouch(seekBar: SeekBar?) {}
            override fun onStopTrackingTouch(seekBar: SeekBar?) {}
        })

        // ── Showcase ──────────────────────────────────────────────────────
        showcaseContainer.addView(makeSectionLabel("Inline layout · baseline alignment"))
        for (row in showcaseInlineRows) {
            addInlineRowParsed(showcaseContainer, row, fontSizeDp)
        }
        showcaseContainer.addView(makeDivider())
        for ((label, latex) in showcaseBlocks) {
            showcaseContainer.addView(makeSectionLabel(label))
            showcaseContainer.addView(makeCenteredMathView(latex, fontSizeDp))
            showcaseContainer.addView(makeDivider())
        }

        // ── Preset block formulas ──────────────────────────────────────────
        for ((name, latex) in formulas) {
            formulaExamplesContainer.addView(makeSectionLabel(name))
            formulaExamplesContainer.addView(makeMathView(latex, fontSizeDp))
        }

        // ── Inline mix ────────────────────────────────────────────────────
        for ((prefix, latex, suffix) in inlineFormulas) {
            addInlineRow(inlineMixContainer, prefix, latex, suffix, fontSizeDp)
        }

        // ── Multiline formulas ────────────────────────────────────────────
        for ((description, latex) in multilineFormulas) {
            multilineMixContainer.addView(makeDescriptionLabel(description))
            multilineMixContainer.addView(makeCenteredMathView(latex, fontSizeDp))
            multilineMixContainer.addView(makeDivider())
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
    }

    // ── Inline row helpers ────────────────────────────────────────────────────

    /**
     * Parses [text] for `$...$` inline math markers, renders each math segment
     * as a [RaTeXSpan] and assembles them all into a single [TextView] for
     * seamless text-and-formula flow with baseline alignment.
     */
    private fun addInlineRowParsed(container: LinearLayout, text: String, sizeDp: Float) {
        val tv = TextView(this).apply {
            this.text = text.replace(Regex("\\$[^$]+\\$"), "…")
            setTextAppearance(android.R.style.TextAppearance_Medium)
            setPadding(0, dp(8f), 0, dp(8f))
            setLineSpacing(dp(6f).toFloat(), 1f)
        }
        container.addView(tv)

        scope.launch {
            try {
                val parts = text.split("$")
                val ssb = SpannableStringBuilder()
                for ((i, part) in parts.withIndex()) {
                    if (part.isEmpty()) continue
                    if (i % 2 == 0) {
                        ssb.append(part)
                    } else {
                        val span = RaTeXSpan.create(this@MainActivity, part, sizeDp)
                        val start = ssb.length
                        ssb.append("\u200B")
                        ssb.setSpan(span, start, ssb.length, 0)
                    }
                }
                tv.text = ssb
            } catch (e: Exception) {
                tv.text = text.replace(Regex("\\$[^$]+\\$"), "[?]")
            }
        }
    }

    /**
     * Renders [latex] as a [RaTeXSpan] and inserts it between [prefix] and
     * [suffix] inside a single [TextView], producing inline text-and-formula flow.
     */
    private fun addInlineRow(
        container: LinearLayout,
        prefix: String,
        latex: String,
        suffix: String,
        sizePx: Float,
    ) {
        val tv = TextView(this).apply {
            text = "$prefix…$suffix"
            setTextAppearance(android.R.style.TextAppearance_Medium)
            setPadding(0, dp(8f), 0, dp(8f))
            setLineSpacing(dp(6f).toFloat(), 1f)
        }
        container.addView(tv)

        scope.launch {
            try {
                val span = RaTeXSpan.create(this@MainActivity, latex, sizePx)
                val ssb = SpannableStringBuilder()
                if (prefix.isNotEmpty()) ssb.append(prefix)
                val spanStart = ssb.length
                ssb.append("\u200B")
                ssb.setSpan(span, spanStart, ssb.length, 0)
                if (suffix.isNotEmpty()) ssb.append(suffix)
                tv.text = ssb
            } catch (e: Exception) {
                tv.text = "$prefix [render error] $suffix"
            }
        }
    }

    // ── UI builder helpers ────────────────────────────────────────────────────

    private fun dp(value: Float) =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value, resources.displayMetrics).toInt()

    private fun makeSectionLabel(text: String) = TextView(this).apply {
        this.text = text
        setTextAppearance(android.R.style.TextAppearance_Small)
        setPadding(0, dp(12f), 0, dp(4f))
    }

    private fun makeDescriptionLabel(text: String) = TextView(this).apply {
        this.text = text
        setTextAppearance(android.R.style.TextAppearance_Medium)
        setPadding(0, dp(12f), 0, dp(6f))
        setLineSpacing(0f, 1.3f)
    }

    private fun makeMathView(latex: String, sizePx: Float) = RaTeXView(this).apply {
        this.latex = latex
        this.fontSize = sizePx
        onError = { e: RaTeXException ->
            Toast.makeText(context, e.message, Toast.LENGTH_SHORT).show()
        }
    }

    private fun makeCenteredMathView(latex: String, sizePx: Float): LinearLayout {
        val container = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(0, dp(4f), 0, dp(4f))
        }
        container.addView(makeMathView(latex, sizePx))
        return container
    }

    private fun makeDivider() = android.view.View(this).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(0.5f)).also {
            it.topMargin = dp(8f)
        }
        setBackgroundColor(0x22000000)
    }

    private fun updateFontSizeLabel(label: TextView, pt: Int) {
        label.text = getString(R.string.font_size_label, pt)
    }
}
