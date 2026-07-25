/* Entry point of the Android build of Fractal.
 *
 * Pixiewood packages a native executable built by Meson, whose `main()` is
 * expected to run the GApplication. Fractal itself is written in Rust, so this
 * shim only forwards to the Rust library, which calls `g_application_run()`.
 */

extern int fractal_main (int argc, char **argv);

int
main (int    argc,
      char **argv,
      char **envp)
{
  (void) envp;

  return fractal_main (argc, argv);
}
