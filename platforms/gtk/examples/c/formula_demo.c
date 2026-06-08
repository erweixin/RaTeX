#include <gtk/gtk.h>
#include <ratex-gtk.h>

static void activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;

    RatexFormula *ratex_formula = ratex_formula_new();
    if (ratex_formula == NULL) {
        g_error("Failed to create RatexFormula");
    }

    GtkWidget *formula = GTK_WIDGET(ratex_formula);
    g_object_set(
        formula,
        "latex", "\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}",
        "font-size", 36.0,
        "display-mode", TRUE,
        NULL);

    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
    gtk_widget_set_margin_top(box, 24);
    gtk_widget_set_margin_bottom(box, 24);
    gtk_widget_set_margin_start(box, 24);
    gtk_widget_set_margin_end(box, 24);
    gtk_box_append(GTK_BOX(box), formula);

    GtkWidget *window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "RaTeX GTK4 C Demo");
    gtk_window_set_default_size(GTK_WINDOW(window), 900, 240);
    gtk_window_set_child(GTK_WINDOW(window), box);
    gtk_window_present(GTK_WINDOW(window));
}

int main(int argc, char **argv) {
    GtkApplication *app = gtk_application_new("io.ratex.demo.gtk4.c", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
