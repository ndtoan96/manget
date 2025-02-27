package main

import "C"
import (
	"archive/zip"
	"bytes"
	"context"
	"unsafe"

	"github.com/pgaskin/kepubify/v4/kepub"
)

//export Convert
func Convert(input_raw *byte, lenIn int, buf *byte, lenBuf int) int {
	input := unsafe.Slice(input_raw, lenIn)
	var opts []kepub.ConverterOption
	opts = append(opts, kepub.ConverterOptionCharset("utf-8"))
	opts = append(opts, kepub.ConverterOptionDummyTitlepage(false))
	converter := kepub.NewConverterWithOptions(opts...)
	zipReader, err := zip.NewReader(bytes.NewReader(input), int64(len(input)))
	if err != nil {
		return -1
	}
	var output bytes.Buffer
	if err := converter.Convert(context.Background(), &output, zipReader); err != nil {
		return -1
	}
	buffer := unsafe.Slice(buf, lenBuf)
	return copy(buffer, output.Bytes())
}

func main() {}
