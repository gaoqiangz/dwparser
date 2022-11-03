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
        )group(level=1   header.height=0  atr=test(123)    trailer.height=76 by= (  "col1",   "col2" ))
        line(band=foreground x1="0")
        compute(band=trailer.5 alignment="2" expression="count(jw_no for group 5 )+~"件~""border="0"  )
        "#;
    let dw = DWSyntax::parse(dwsyn).unwrap();
    println!("\r\nAST:\r\n{:#?})", dw);
    println!("\r\nToString:\r\n{})", dw);
}
