exception TestFail

let _ =
  let open Stubs in
  let v1 = { x = 1.; y= 2.; z = 3. } in
  let v2 = { x = 4.; y= 5.; z = 6. } in
  let v3 = { x = 7.; y= 8.; z = 9. } in
  let v4 = add_vecs v1 v3 in
  let v5 = add_vecs v4 v2 in
  let sum = sum_vecs (Many [| v1; v2; v3; v4; v5 |]) { bound = 3 } in
  Printf.printf "[ %.2f; %.2f; %.2f ]\n" sum.x sum.y sum.z;

  let open Stubs in
  try
    test_panic ();
    raise TestFail
  with
  | RustFFI err -> Printf.printf "got rust ffi panic message: %s\n" err
