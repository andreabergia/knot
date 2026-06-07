(module
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 4096))

  (data (i32.const 16) "{\"abi_version\":1,\"id\":\"knot/ts-debugger\",\"name\":\"No debugger\",\"severity\":\"warning\"}")
  (data (i32.const 256) "\"kind\":\"debugger\",\"span\":")
  (data (i32.const 512) "{\"rule_id\":\"knot/ts-debugger\",\"severity\":\"warning\",\"message\":\"Unexpected debugger statement.\",\"span\":")

  (func $pack (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $matches_marker (param $ptr i32) (result i32)
    (local $index i32)
    (block $mismatch
      (loop $next
        (br_if $mismatch
          (i32.ne
            (i32.load8_u (i32.add (local.get $ptr) (local.get $index)))
            (i32.load8_u (i32.add (i32.const 256) (local.get $index)))))
        (local.set $index (i32.add (local.get $index) (i32.const 1)))
        (br_if $next (i32.lt_u (local.get $index) (i32.const 25)))
        (return (i32.const 1))))
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
    (call $pack (i32.const 16) (i32.const 83)))

  (func (export "knot_check") (param $ptr i32) (param $len i32) (result i64)
    (local $cursor i32)
    (local $limit i32)
    (local $output i32)
    (local $output_cursor i32)
    (local $span_start i32)
    (local $span_end i32)
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
          (i32.gt_u (i32.add (local.get $cursor) (i32.const 25)) (local.get $limit)))

        (if (call $matches_marker (local.get $cursor))
          (then
            (local.set $span_start (i32.add (local.get $cursor) (i32.const 25)))
            (local.set $span_end (local.get $span_start))
            (block $span_done
              (loop $find_span_end
                (br_if $span_done
                  (i32.eq
                    (i32.load8_u (local.get $span_end))
                    (i32.const 125)))
                (local.set $span_end (i32.add (local.get $span_end) (i32.const 1)))
                (br $find_span_end)))

            (if (i32.gt_u (local.get $diagnostic_count) (i32.const 0))
              (then
                (i32.store8 (local.get $output_cursor) (i32.const 44))
                (local.set $output_cursor
                  (i32.add (local.get $output_cursor) (i32.const 1)))))

            (call $copy (local.get $output_cursor) (i32.const 512) (i32.const 101))
            (local.set $output_cursor
              (i32.add (local.get $output_cursor) (i32.const 101)))
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
              (i32.add (local.get $diagnostic_count) (i32.const 1)))
            (local.set $cursor (local.get $span_end))))

        (local.set $cursor (i32.add (local.get $cursor) (i32.const 1)))
        (br $scan)))

    (i32.store8 (local.get $output_cursor) (i32.const 93))
    (local.set $output_cursor (i32.add (local.get $output_cursor) (i32.const 1)))
    (call $pack
      (local.get $output)
      (i32.sub (local.get $output_cursor) (local.get $output))))
)
