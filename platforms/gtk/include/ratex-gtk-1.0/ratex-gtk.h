/* ratex-gtk.h - GTK4/GObject widget API for RaTeX */

#ifndef RATEX_GTK_H
#define RATEX_GTK_H

#include <gtk/gtk.h>

G_BEGIN_DECLS

#define RATEX_TYPE_FORMULA (ratex_formula_get_type())

G_DECLARE_FINAL_TYPE(RatexFormula, ratex_formula, RATEX, FORMULA, GtkWidget)

/* RatexFormula is implemented as a gtk-rs GtkWidget subclass. GTK must be
 * initialized on the main thread before querying RatexFormula's type or
 * constructing widgets.
 * Returns NULL if GTK has not been initialized.
 */
RatexFormula *ratex_formula_new(void);

G_END_DECLS

#endif /* RATEX_GTK_H */
