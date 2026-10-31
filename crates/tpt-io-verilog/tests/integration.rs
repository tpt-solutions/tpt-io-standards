//! Integration tests for the structural Verilog reader and writer.

use tpt_io_verilog::{parse, write, Direction, VerilogError};

const NETLIST: &[u8] = b"// top level for the reader tests
module adder (a, b, sum, carry);
  input [3:0] a, b;
  output [3:0] sum;
  output carry;
  wire n1, n2;
  /* two half adders and an OR */
  XOR2 u1 (.A(a[0]), .B(b[0]), .Y(sum[0]));
  AND2 u2 (.A(a[0]), .B(b[0]), .Y(n1));
  OR2  u3 (n1, n2, carry);
  assign n2 = 1'b0;
endmodule

module wrapper (x, y);
  input x;
  output y;
  adder core (.a(x), .b(x), .sum(), .carry(y));
endmodule
";

#[test]
fn reads_modules_ports_wires_and_instances() {
    let n = parse(NETLIST).unwrap();
    assert_eq!(n.modules.len(), 2);
    let adder = &n.modules[0];
    assert_eq!(adder.name, "adder");
    assert_eq!(adder.header_ports, ["a", "b", "sum", "carry"]);
    assert_eq!(adder.ports.len(), 4);
    assert_eq!(adder.ports[0].direction, Direction::Input);
    assert_eq!(adder.ports[0].range.as_deref(), Some("[3:0]"));
    assert_eq!(adder.wires.len(), 2);
    assert_eq!(adder.instances.len(), 3);
    assert_eq!(adder.instances[0].cell, "XOR2");
    assert_eq!(adder.instances[0].connections[0].net, "a[0]");
    assert_eq!(adder.instances[2].connections[0].port, None);
    assert_eq!(adder.assigns, [("n2".to_string(), "1'b0".to_string())]);
}

#[test]
fn empty_named_connections_are_kept() {
    let n = parse(NETLIST).unwrap();
    let core = &n.modules[1].instances[0];
    assert_eq!(core.connections[2].port.as_deref(), Some("sum"));
    assert_eq!(core.connections[2].net, "");
}

#[test]
fn write_then_parse_is_identity() {
    let n = parse(NETLIST).unwrap();
    let text = write(&n);
    assert_eq!(parse(&text).unwrap(), n);
}

#[test]
fn ansi_headers_and_escaped_names_read() {
    let src = b"module m (input wire clk, output reg q);\nendmodule\n";
    let err = parse(src);
    assert!(matches!(err, Err(VerilogError::Unsupported(_))) || err.is_ok());
    let n = parse(b"module m (input clk, output q); endmodule").unwrap();
    assert_eq!(n.modules[0].ports.len(), 2);
    let esc = parse(b"module m (\\a+b , y); output y; INV \\u$1 (.A(\\a+b ), .Y(y)); endmodule").unwrap();
    assert_eq!(esc.modules[0].instances[0].name, "\\u$1");
}

#[test]
fn behavioural_code_is_rejected_not_approximated() {
    let src = b"module m (a); input a; always @(a) begin end endmodule";
    assert!(matches!(parse(src), Err(VerilogError::Unsupported(_))));
}

#[test]
fn malformed_netlists_are_errors() {
    assert!(parse(b"wire x;").is_err());
    assert!(parse(b"module m (a); input a;").is_err());
    assert!(parse(b"module m (a; endmodule").is_err());
    assert!(parse(b"module m; /* unterminated").is_err());
    assert!(parse(b"module m; ! endmodule").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 47;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"module endmodule input wire ;(),.[]:a1 \n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
