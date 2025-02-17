#![allow(unused_mut)]

use dwparser::DWSyntax;

fn main() {
    let dwsyn = r#"
        release 19;datawindow(units=0 timer_interval=""  test = '' color=1073741824 print.margin.bottom = '96' print.canusedefaultprinter=yes  )
        header(height=564 color="536870912"gradient.repetition.length="100" )
        table(column=(type= char(80  ) updatewhereclause=yes name=col1 dbname="col1" )
            column=(type=char ( 80) updatewhereclause=yes name=col2 dbname="col2" )
 column=(type=char(10)updatewhereclause=yes name=check_status dbname="check_status" values="盘盈	P/盘亏	L/正常	N/" )
        retrieve="  SELECT col1,col2,col3 
        FROM test "
            arguments = ( ("a", string ), ( "b", string)  )
            sort= "row_num A "
        )group(level=1   header.height=100  atr=test(123)    trailer.height=76 by= (  "col1",   "col2" ))
        line(band=foreground x1="0")
        compute(name=compute_1 band=trailer.5 alignment="2" expression="count(jw_no for group 5 )+~"件~""border="0"  )
        column(name=col1 id=1 alignment="2" )
        column(name=col2 id=2 alignment="2" )
        column(name=col3 id=3 alignment="2" )
        "#;
    let mut dw = DWSyntax::parse(dwsyn).unwrap();
    println!("\r\nAST:\r\n{:#?}", dw);
    #[cfg(feature = "serde_support")]
    println!("\r\nAST_JSON:\r\n{}", serde_json::to_string_pretty(&dw).unwrap());

    #[cfg(feature = "query")]
    {
        println!("\r\nDescribe:\r\n");
        println!("datawindow.color: {}", dw.describe("datawindow.color"));
        println!("datawindow.header.color: {}", dw.describe("datawindow.header.color"));
        println!("datawindow.header.height: {}", dw.describe("datawindow.header.height"));
        println!("datawindow.header.1.height: {}", dw.describe("datawindow.header.1.height"));
        println!("datawindow.group.1.by: {}", dw.describe("datawindow.group.1.by"));
        println!("datawindow.group.1.prop: {}", dw.describe("datawindow.group.1.prop"));
        println!("datawindow.table.arguments: {}", dw.describe("datawindow.table.arguments"));
        println!("datawindow.table.column.1.type: {}", dw.describe("datawindow.table.column.1.type"));
        println!("datawindow.table.column.col2.type: {}", dw.describe("datawindow.table.column.col2.type"));
        println!("compute_1.expression: {}", dw.describe("compute_1.expression"));
        println!("datawindow.column.count: {}", dw.describe("datawindow.column.count"));
        println!("datawindow.objects: {}", dw.describe("datawindow.objects"));

        //modify
        dw.modify("datawindow.color='red'");
        dw.modify("datawindow.header.height=200");
        dw.modify("datawindow.header.1.height=200");
        dw.modify("datawindow.table.column.col1.type=long");
        dw.modify("datawindow.group.1.prop='test prop'");
        dw.modify("datawindow.group.1.level=1");
        dw.modify("compute_1.expression='getrow()'");
        dw.modify("destroy compute_1 destroy datawindow.table.column.col1");
        dw.modify(r#"create compute(name=compute_2 level=2 band=trailer.5 alignment="2"   )"#);
        dw.modify(r#"create group(level=2   header.height=322  atr=test(123)    trailer.height=76 by= (  "col1",   "col2" ))"#);
        dw.modify(r#"create group(level=3   header.height=244  atr=test(123)    trailer.height=76 by= (  "col1",   "col2" ))"#);
    }

    println!("\r\nToString:\r\n{}", dw);
}
