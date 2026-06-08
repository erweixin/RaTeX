[CCode (cprefix = "Ratex", gir_namespace = "RatexGtk", gir_version = "1.0", lower_case_cprefix = "ratex_")]
namespace RatexGtk {
    [CCode (cheader_filename = "ratex-gtk.h", cname = "RatexFormula", type_id = "ratex_formula_get_type ()")]
    public class Formula : Gtk.Widget {
        [CCode (cname = "ratex_formula_new")]
        public Formula ();

        [NoAccessorMethod]
        public string latex { owned get; set; }
        [NoAccessorMethod]
        public bool display_mode { get; set; }
        [NoAccessorMethod]
        public double font_size { get; set; }
        [NoAccessorMethod]
        public double padding { get; set; }
        [NoAccessorMethod]
        public Gdk.RGBA? color { owned get; set; }
        [NoAccessorMethod]
        public string? font_dir { owned get; set; }
        [NoAccessorMethod]
        public string? error_message { owned get; }
    }
}
