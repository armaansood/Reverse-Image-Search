#!/usr/bin/env python
##############################################################################
# Copyright (c) 2012 Hajime Nakagami<nakagami@gmail.com>
# All rights reserved.
# Licensed under the New BSD License
# (http://www.freebsd.org/copyright/freebsd-license.html)
#
# A image viewer. Require Pillow ( https://pypi.python.org/pypi/Pillow/ ).
##############################################################################
from PIL import Image, ImageFilter
import requests
import io
import json

try:
    from Tkinter import *
    import tkFileDialog as filedialog
except ImportError:
    from tkinter import *
    from tkinter import filedialog
import PIL.ImageTk

class App(Frame):
    def chg_image(self):
        if self.im.mode == "1": # bitmap image
            self.img = PIL.ImageTk.BitmapImage(self.im, foreground="white")
        else:              # photo image
            self.img = PIL.ImageTk.PhotoImage(self.im)
        self.la.config(image=self.img, bg="#000000",
            width=self.img.width(), height=self.img.height())

    def open(self):
        filename = filedialog.askopenfilename()
        if filename != "":
            self.im = PIL.Image.open(filename)
        self.chg_image()
        self.num_page=0
        self.num_page_tv.set(str(self.num_page+1))

    def blur_img(self):
        self.im = self.im.filter(ImageFilter.GaussianBlur(3))
        self.img = PIL.ImageTk.PhotoImage(self.im)
        self.la.config(image=self.img, bg="#000000",
            width=self.img.width(), height=self.img.height())

    def query_img(self):
        buf = io.BytesIO()
        self.im.save(buf, 'JPEG')
        res = requests.post(url='http://localhost:1080',
                    data=buf.getvalue(),
                    headers={'Content-Type': 'application/octet-stream'})
        res = json.loads(res.text)
        for _, frame in self.results:
            frame.destroy()
        self.results = []
        for [path, score] in res:
            frame = Frame(self.res_fram)
            im = PIL.Image.open(path)
            im.thumbnail((64,64))
            img = PIL.ImageTk.PhotoImage(im)
            la = Label(frame)
            la.config(image=img, bg="#000000",
                width=img.width(), height=img.height())
            lt = Label(frame)
            lt.config(text="{0:.2f}".format(score))
            la.pack(side=TOP)
            lt.pack(side=TOP)
            self.results.append((img, frame))
        for (_, frame) in self.results:
            frame.pack(side=LEFT)

    def __init__(self, master=None):
        Frame.__init__(self, master)
        self.master.title('Image Viewer')

        self.num_page=0
        self.num_page_tv = StringVar()
        self.results = []

        fram = Frame(self, width = 600, height = 600)
        Button(fram, text="Open File", command=self.open).pack(side=LEFT)
        Button(fram, text="Blur", command=self.blur_img).pack(side=LEFT)
        Button(fram, text="Query", command=self.query_img).pack(side=LEFT)
        fram.pack(side=TOP, fill=BOTH)
        res_fram = Frame(self, height = 70, width = 70 * 3)
        res_fram.pack(side=BOTTOM, fill=BOTH)
        self.res_fram = res_fram

        self.la = Label(self)
        self.la.pack()

        self.pack()

if __name__ == "__main__":
    app = App(); app.mainloop()
