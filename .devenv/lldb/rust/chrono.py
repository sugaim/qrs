from datetime import datetime, timedelta

def naive_date(valobj, internal_dict):
    ymdf = valobj.GetChildMemberWithName('ymdf').GetValue()
    year = int(ymdf) >> 13  
    day_of_year = (int(ymdf) & 8191) >> 4
    date = datetime(year -1, 12, 31) + timedelta(days=day_of_year)
    return date.strftime('%Y-%m-%d')

def naive_time(valobj, internal_dict):
    secs = valobj.GetChildMemberWithName('secs').GetValue()
    nano = valobj.GetChildMemberWithName('frac').GetValue()
    h, m, s = int(secs) // 3600, (int(secs) // 60) % 60, int(secs) % 60
    return f"{h:02}:{m:02}:{s:02}.{int(nano):09}"

def naive_datetime(valobj, internal_dict):
    ymdf = valobj.GetChildMemberWithName('date').GetChildMemberWithName('ymdf').GetValue()
    secs = valobj.GetChildMemberWithName('time').GetChildMemberWithName('secs').GetValue()
    nano = valobj.GetChildMemberWithName('time').GetChildMemberWithName('frac').GetValue()
    year = int(ymdf) >> 13  
    day_of_year = (int(ymdf) & 8191) >> 4
    date = datetime(year -1, 12, 31) + timedelta(days=day_of_year) + timedelta(seconds=int(secs))
    subsec = int(nano) // 1000000
    return date.strftime('%Y-%m-%dT%H:%M:%S') + f".{subsec:03}"

def datetime_fixed(valobj, internal_dict):
    ymdf = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('date').GetChildMemberWithName('ymdf').GetValue()
    secs = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('time').GetChildMemberWithName('secs').GetValue()
    nano = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('time').GetChildMemberWithName('frac').GetValue()
    offset = int(valobj.GetChildMemberWithName('offset').GetChildMemberWithName('local_minus_utc').GetValue())
    year = int(ymdf) >> 13  
    day_of_year = (int(ymdf) & 8191) >> 4
    offset_sign = "+" if 0 <= offset else "-"
    offset_dt = datetime(1900, 1, 1) + timedelta(seconds=abs(offset))
    date = datetime(year -1, 12, 31) + timedelta(days=day_of_year) + timedelta(seconds=int(secs) + offset)
    subsec = int(nano) // 1000000
    return date.strftime('%Y-%m-%dT%H:%M:%S') + f".{subsec:03}" + offset_sign + offset_dt.strftime('%H:%M')

def datetime_utc(valobj, internal_dict):
    ymdf = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('date').GetChildMemberWithName('ymdf').GetValue()
    secs = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('time').GetChildMemberWithName('secs').GetValue()
    nano = valobj.GetChildMemberWithName('datetime').GetChildMemberWithName('time').GetChildMemberWithName('frac').GetValue()
    year = int(ymdf) >> 13  
    day_of_year = (int(ymdf) & 8191) >> 4
    date = datetime(year -1, 12, 31) + timedelta(days=day_of_year) + timedelta(seconds=int(secs))
    subsec = int(nano) // 1000000
    return date.strftime('%Y-%m-%dT%H:%M:%S') + f".{subsec:03}" + "Z"
    