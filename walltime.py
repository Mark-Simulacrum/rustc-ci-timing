import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
import csv
import itertools
import collections
import statistics as stats
from datetime import datetime
from collections import namedtuple

Point = namedtuple('Point', ['time', 'cpu'])
DatePoint = namedtuple('DatePoint', ['date', 'time', 'cpu'])

by_builder = {}
dates = set()
with open('data.csv') as f:
    for line in f:
        c = csv.reader([line], delimiter=',')
        for row in c:
            date = datetime.fromisoformat(row[1])
            dates.add(date)
            by_builder.setdefault(row[2], {})[date] = Point(
                time=int(row[3])/60.0/60,
                cpu=float(row[4]),
            )

fig, (ax, ax2) = plt.subplots(2)

dates = list(sorted(dates))

# Get the top 5 builders by the maximum time in the last commit.
time_points = []
cpu_points = []
for builder in by_builder.keys():
    last_date = dates[-1]
    pt = by_builder[builder].get(last_date)
    if pt:
        time_points.append((pt.time, builder))
        cpu_points.append((pt.cpu, builder))
top_5_time = [pt[1] for pt in list(sorted(time_points))[-5:]]
min_5_cpu = [pt[1] for pt in list(sorted(cpu_points))[:5]]

for builder in by_builder.keys():
    new_data = []
    for date in dates:
        if date in by_builder[builder]:
            new_data.append(DatePoint(
                date=date,
                cpu=by_builder[builder][date].cpu,
                time=by_builder[builder][date].time,
            ))
        else:
            new_data.append(DatePoint(
                date=date,
                cpu=None,
                time=None,
            ))
    by_builder[builder] = new_data

def downsample_data(data, idx, downsampler=lambda a: stats.median(a), length=12):
    dates = []
    new_data = []
    window = collections.deque(
        map(lambda v: v[idx], itertools.islice(data, length)),
        maxlen=length
    )
    window_date = collections.deque(
        map(lambda v: v[0], itertools.islice(data, length)),
        maxlen=length
    )
    for entry in data:
        window_date.append(entry[0])
        window.append(entry[idx])
        filtered = list(filter(lambda x: x is not None, window))
        if len(filtered) > 0:
            new_data.append(downsampler(filtered))
            dates.append(window_date[-1])
    return (dates, new_data)

for key in by_builder.keys():
    if key in top_5_time:
        (dates, data) = downsample_data(by_builder[key], 1)
        ax.plot(dates, data, label=key)
        (dates, data) = downsample_data(by_builder[key], 2, length=1)
        ax2.plot(dates, data, label=key)

ax.set(ylabel = "Hours")
ax2.set(ylabel = "CPU usage")

for a in [ax, ax2]:
    a.legend(loc='upper left')
    minor = mdates.RRuleLocator(mdates.rrulewrapper(mdates.WEEKLY))
    a.xaxis.set_minor_locator(minor)
    a.format_xdata = mdates.DateFormatter('%Y-%m-%d')
    a.grid()

plt.show()
