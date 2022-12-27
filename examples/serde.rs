#![allow(unused_mut)]

use dwparser::DWSyntax;

fn main() {
    let syn_json = r#"{
    "name": null,
    "comment": null,
    "version": 19.0,
    "datawindow": {
        "units": {
        "number": 0.0
        },
        "timer_interval": {
        "dqt_str": ""
        },
        "test": {
        "sqt_str": ""
        },
        "color": {
        "number": 1073741824.0
        },
        "print.margin.bottom": {
        "sqt_str": "96"
        },
        "print.canusedefaultprinter": {
        "lit": "yes"
        }
    },
    "header": {
        "height": {
        "number": 564.0
        },
        "color": {
        "dqt_str": "536870912"
        },
        "gradient.repetition.length": {
        "dqt_str": "100"
        }
    },
    "summary": {},
    "footer": {},
    "detail": {},
    "table": {
        "columns": [
        {
            "name": "col1",
            "values": {
            "type": {
                "lit": "char(80  )"
            },
            "updatewhereclause": {
                "lit": "yes"
            },
            "name": {
                "lit": "col1"
            },
            "dbname": {
                "dqt_str": "col1"
            }
            }
        },
        {
            "name": "col2",
            "values": {
            "type": {
                "lit": "char ( 80)"
            },
            "updatewhereclause": {
                "lit": "yes"
            },
            "name": {
                "lit": "col2"
            },
            "dbname": {
                "dqt_str": "col2"
            }
            }
        },
        {
            "name": "check_status",
            "values": {
            "type": {
                "lit": "char(10)"
            },
            "updatewhereclause": {
                "lit": "yes"
            },
            "name": {
                "lit": "check_status"
            },
            "dbname": {
                "dqt_str": "check_status"
            },
            "values": {
                "dqt_str": "盘盈\tP/盘亏\tL/正常\tN/"
            }
            }
        }
        ],
        "values": {
        "retrieve": {
            "dqt_str": "  SELECT col1,col2,col3 \n        FROM test "
        },
        "arguments": {
            "list": [
            {
                "list": [
                {
                    "dqt_str": "a"
                },
                {
                    "lit": "string"
                }
                ]
            },
            {
                "list": [
                {
                    "dqt_str": "b"
                },
                {
                    "lit": "string"
                }
                ]
            }
            ]
        },
        "sort": {
            "dqt_str": "row_num A "
        }
        }
    },
    "data": [],
    "items": [
        {
        "kind": "group",
        "name": null,
        "id": null,
        "values": {
            "level": {
            "number": 1.0
            },
            "header.height": {
            "number": 100.0
            },
            "atr": {
            "lit": "test(123)"
            },
            "trailer.height": {
            "number": 76.0
            },
            "by": {
            "list": [
                {
                "dqt_str": "col1"
                },
                {
                "dqt_str": "col2"
                }
            ]
            }
        }
        },
        {
        "kind": "line",
        "name": null,
        "id": null,
        "values": {
            "band": {
            "lit": "foreground"
            },
            "x1": {
            "dqt_str": "0"
            }
        }
        },
        {
        "kind": "compute",
        "name": "compute_1",
        "id": null,
        "values": {
            "name": {
            "lit": "compute_1"
            },
            "band": {
            "lit": "trailer.5"
            },
            "alignment": {
            "dqt_str": "2"
            },
            "expression": {
            "dqt_str": "count(jw_no for group 5 )+~\"件~\""
            },
            "border": {
            "dqt_str": "0"
            }
        }
        }
    ]
    }"#;
    let dw = serde_json::from_str::<DWSyntax>(&syn_json).unwrap();
    println!("\r\nAST:\r\n{:#?}", dw);

    println!("\r\nToString:\r\n{}", dw);
}
