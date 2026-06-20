(module
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 4096))

  (data (i32.const 16) "{\"abi_version\":1,\"id\":\"knot/py-mutable-default-arg\",\"name\":\"No mutable defaults\",\"severity\":\"warning\"}")
  (data (i32.const 160) "\"kind\":\"parameter_default\",\"span\":")
  (data (i32.const 256) "\"literal\":\"")
  (data (i32.const 512) "{\"rule_id\":\"knot/py-mutable-default-arg\",\"severity\":\"warning\",\"message\":\"Mutable default argument.\",\"span\":")

  (func $pack (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $matches_marker (param $ptr i32) (param $marker i32) (param $marker_len i32) (result i32)
    (local $index i32)
    (block $mismatch
      (loop $next
        (br_if $mismatch
          (i32.ne
            (i32.load8_u (i32.add (local.get $ptr) (local.get $index)))
            (i32.load8_u (i32.add (local.get $marker) (local.get $index)))))
        (local.set $index (i32.add (local.get $index) (i32.const 1)))
        (br_if $next (i32.lt_u (local.get $index) (local.get $marker_len)))
        (return (i32.const 1))))
    (i32.const 0))

  (func $is_mutable_literal (param $ptr i32) (result i32)
    (local $ch i32)
    (local.set $ch (i32.load8_u (i32.add (local.get $ptr) (i32.const 11))))
    (if (i32.eq (local.get $ch) (i32.const 108)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $ch) (i32.const 100)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $ch) (i32.const 115)) (then (return (i32.const 1))))
    (i32.const 0))

  (func $copy (param $destination i32) (param $source i32) (param $length i32)
    (memory.copy
      (local.get $destination)
      (local.get $source)
      (local.get $length)))

  (func (export "knot_alloc") (param $len i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $len)))
    (local.get $ptr))

  (func (export "knot_dealloc") (param $ptr i32) (param $len i32))

  (func (export "knot_metadata") (result i64)
    (call $pack (i32.const 16) (i32.const 102)))

  (func (export "knot_check") (param $ptr i32) (param $len i32) (result i64)
    (local $cursor i32)
    (local $limit i32)
    (local $output i32)
    (local $output_cursor i32)
    (local $span_start i32)
    (local $span_end i32)
    (local $search_cursor i32)
    (local $diagnostic_count i32)

    (local.set $cursor (local.get $ptr))
    (local.set $limit (i32.add (local.get $ptr) (local.get $len)))
    (local.set $output (i32.const 65536))
    (local.set $output_cursor (local.get $output))
    (i32.store8 (local.get $output_cursor) (i32.const 91))
    (local.set $output_cursor (i32.add (local.get $output_cursor) (i32.const 1)))

    (block $done
      (loop $scan
        (br_if $done
          (i32.gt_u (i32.add (local.get $cursor) (i32.const 34)) (local.get $limit)))

        (if (call $matches_marker (local.get $cursor) (i32.const 160) (i32.const 34))
          (then
            (local.set $span_start (i32.add (local.get $cursor) (i32.const 34)))
            (local.set $span_end (local.get $span_start))

            (block $span_done
              (loop $find_span_end
                (local.set $span_end (i32.add (local.get $span_end) (i32.const 1)))
                (br_if $span_done
                  (i32.eq
                    (i32.load8_u (local.get $span_end))
                    (i32.const 125)))
                (br $find_span_end)))

            (local.set $search_cursor (i32.add (local.get $span_end) (i32.const 1)))

            (block $literal_done
              (loop $find_literal
                (if (i32.ge_u (local.get $search_cursor) (local.get $limit))
                  (then (br $literal_done)))

                (if (i32.eq
                      (i32.load8_u (local.get $search_cursor))
                      (i32.const 125))
                  (then
                    (if (i32.eq
                          (i32.load8_u (i32.add (local.get $search_cursor) (i32.const 1)))
                          (i32.const 125))
                      (then (br $literal_done)))))

                (if (call $matches_marker (local.get $search_cursor) (i32.const 256) (i32.const 11))
                  (then
                    (if (call $is_mutable_literal (local.get $search_cursor))
                      (then
                        (if (i32.gt_u (local.get $diagnostic_count) (i32.const 0))
                          (then
                            (i32.store8 (local.get $output_cursor) (i32.const 44))
                            (local.set $output_cursor
                              (i32.add (local.get $output_cursor) (i32.const 1)))))

                        (call $copy (local.get $output_cursor) (i32.const 512) (i32.const 107))
                        (local.set $output_cursor
                          (i32.add (local.get $output_cursor) (i32.const 107)))
                        (call $copy
                          (local.get $output_cursor)
                          (local.get $span_start)
                          (i32.add
                            (i32.sub (local.get $span_end) (local.get $span_start))
                            (i32.const 1)))
                        (local.set $output_cursor
                          (i32.add
                            (local.get $output_cursor)
                            (i32.add
                              (i32.sub (local.get $span_end) (local.get $span_start))
                              (i32.const 1))))
                        (i32.store8 (local.get $output_cursor) (i32.const 125))
                        (local.set $output_cursor
                          (i32.add (local.get $output_cursor) (i32.const 1)))
                        (local.set $diagnostic_count
                          (i32.add (local.get $diagnostic_count) (i32.const 1)))))
                      (br $literal_done)))

                (local.set $search_cursor (i32.add (local.get $search_cursor) (i32.const 1)))
                (br $find_literal)))

            (local.set $cursor (local.get $span_end))))

        (local.set $cursor (i32.add (local.get $cursor) (i32.const 1)))
        (br $scan)))

    (i32.store8 (local.get $output_cursor) (i32.const 93))
    (local.set $output_cursor (i32.add (local.get $output_cursor) (i32.const 1)))
    (call $pack
      (local.get $output)
      (i32.sub (local.get $output_cursor) (local.get $output)))))