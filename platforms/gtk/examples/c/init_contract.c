#include <gtk/gtk.h>
#include <ratex-gtk.h>

int main(void) {
    if (ratex_formula_get_type() != G_TYPE_INVALID) {
        g_printerr("ratex_formula_get_type() must be invalid before GTK init\n");
        return 1;
    }

    if (ratex_formula_new() != NULL) {
        g_printerr("ratex_formula_new() must return NULL before GTK init\n");
        return 1;
    }

    gtk_init();

    if (ratex_formula_get_type() == G_TYPE_INVALID) {
        g_printerr("ratex_formula_get_type() must be valid after GTK init\n");
        return 1;
    }

    RatexFormula *formula = ratex_formula_new();
    if (formula == NULL) {
        g_printerr("ratex_formula_new() must construct after GTK init\n");
        return 1;
    }

    g_object_unref(formula);
    return 0;
}
